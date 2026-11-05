//! Git through the user's own `git` command line, so their credentials, SSH
//! keys, hooks and config work exactly as in a terminal. Nothing here ever
//! asks for a password: Git is told not to prompt, and a push that needs
//! credentials fails with Git's own message.
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Patches and logs larger than this are cut off.
const MAX_OUTPUT: usize = 512 * 1024;

/// The PATH of the user's login shell. Apps started from the Dock or Finder
/// get a bare PATH, which would miss Homebrew's git and the tools Git hooks
/// call (node for husky, for one).
fn login_path() -> Option<&'static str> {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    PATH.get_or_init(|| {
        if cfg!(windows) {
            return None;
        }
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let out = Command::new(shell).args(["-l", "-c", "printf %s \"$PATH\""]).output().ok()?;
        let path = String::from_utf8(out.stdout).ok()?;
        (!path.trim().is_empty()).then(|| path.trim().to_string())
    })
    .as_deref()
}

fn git(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(root).env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0").env("LC_ALL", "C");
    if let Some(path) = login_path() {
        cmd.env("PATH", path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashing up for each call.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Runs git and returns its stdout, or its stderr as the error.
fn run(root: &Path, args: &[&str]) -> AppResult<String> {
    let out = git(root).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { AppError::msg("Git isn't installed (or not on PATH)") } else { AppError::Io(e) }
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let err = if err.is_empty() { String::from_utf8_lossy(&out.stdout).trim().to_string() } else { err };
        return Err(AppError::msg(if err.is_empty() { format!("git {} failed", args[0]) } else { err }));
    }
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.len() > MAX_OUTPUT {
        let mut cut = MAX_OUTPUT;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("\n… (cut off)\n");
    }
    Ok(text)
}

/// Like `run`, with stdout and stderr together (push and pull report on stderr).
fn run_all(root: &Path, args: &[&str]) -> AppResult<String> {
    let out = git(root).args(args).output()?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).trim().to_string();
    if out.status.success() { Ok(text) } else { Err(AppError::msg(if text.is_empty() { format!("git {} failed", args[0]) } else { text })) }
}

#[derive(Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub path: String,
    /// The old path of a rename.
    pub from: Option<String>,
    /// added, modified, deleted, renamed, untracked or conflict.
    pub kind: String,
    pub staged: bool,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub repo: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<Change>,
}

fn kind(xy: &str) -> &'static str {
    let (x, y) = (xy.chars().next().unwrap_or('.'), xy.chars().nth(1).unwrap_or('.'));
    match (x, y) {
        ('A', _) => "added",
        ('D', _) | (_, 'D') => "deleted",
        ('R', _) | ('C', _) => "renamed",
        _ => "modified",
    }
}

/// Parses `git status --porcelain=v2 --branch -z`.
pub fn parse_status(out: &str) -> Status {
    let mut s = Status { repo: true, ..Default::default() };
    let mut fields = out.split('\0').peekable();
    while let Some(line) = fields.next() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            s.branch = (rest != "(detached)").then(|| rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            s.upstream = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split(' ') {
                if let Some(n) = part.strip_prefix('+') {
                    s.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    s.behind = n.parse().unwrap_or(0);
                }
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // XY sub mH mI mW hH hI path
            let parts: Vec<&str> = rest.splitn(8, ' ').collect();
            if parts.len() == 8 {
                let xy = parts[0];
                s.changes.push(Change { path: parts[7].into(), from: None, kind: kind(xy).into(), staged: !xy.starts_with('.') });
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // XY sub mH mI mW hH hI Xscore path, then the old path as the next field
            let parts: Vec<&str> = rest.splitn(9, ' ').collect();
            if parts.len() == 9 {
                let from = fields.next().map(String::from);
                s.changes.push(Change { path: parts[8].into(), from, kind: "renamed".into(), staged: !parts[0].starts_with('.') });
            }
        } else if let Some(rest) = line.strip_prefix("u ") {
            if let Some(path) = rest.splitn(10, ' ').nth(9) {
                s.changes.push(Change { path: path.into(), from: None, kind: "conflict".into(), staged: false });
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            s.changes.push(Change { path: path.into(), from: None, kind: "untracked".into(), staged: false });
        }
    }
    s
}

pub fn status(root: &Path) -> AppResult<Status> {
    match run(root, &["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"]) {
        Ok(out) => Ok(parse_status(&out)),
        Err(e) if e.to_string().contains("not a git repository") => Ok(Status::default()),
        Err(e) => Err(e),
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub author: String,
    /// ISO 8601.
    pub date: String,
    pub subject: String,
    /// Not on the upstream branch yet.
    pub local: bool,
}

const LOG_FORMAT: &str = "--pretty=format:%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1e";

pub fn parse_log(out: &str, local: &[&str]) -> Vec<Commit> {
    out.split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches('\n').split('\x1f').collect();
            (f.len() == 5).then(|| Commit {
                local: local.contains(&f[0]),
                hash: f[0].into(),
                short: f[1].into(),
                author: f[2].into(),
                date: f[3].into(),
                subject: f[4].into(),
            })
        })
        .collect()
}

pub fn log(root: &Path, limit: u32) -> AppResult<Vec<Commit>> {
    let n = format!("-n{}", limit.clamp(1, 1000));
    let out = match run(root, &["log", &n, LOG_FORMAT]) {
        Ok(out) => out,
        // A repository without commits yet.
        Err(e) if e.to_string().contains("does not have any commits") => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let unpushed = run(root, &["rev-list", "@{u}..HEAD"]).unwrap_or_default();
    let local: Vec<&str> = unpushed.lines().collect();
    Ok(parse_log(&out, &local))
}

/// The working tree's changes to `path` against the last commit; for a new
/// file, the whole file as added.
pub fn diff(root: &Path, path: &str) -> AppResult<String> {
    let has_head = run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
    if has_head {
        let tracked = run(root, &["diff", "HEAD", "--no-color", "--", path])?;
        if !tracked.is_empty() {
            return Ok(tracked);
        }
    }
    // Untracked (or no commit yet): compare with nothing.
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let out = git(root).args(["diff", "--no-color", "--no-index", "--", null, path]).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A commit's message, files and patch.
pub fn show(root: &Path, hash: &str) -> AppResult<String> {
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::msg("Invalid commit"));
    }
    run(root, &["show", "--no-color", "--stat", "--patch", "--format=fuller", hash])
}

/// Stages everything and commits it.
pub fn commit(root: &Path, message: &str) -> AppResult<String> {
    if message.trim().is_empty() {
        return Err(AppError::msg("Write a commit message first"));
    }
    run(root, &["add", "--all"])?;
    run_all(root, &["commit", "-m", message])
}

/// Pushes the branch, setting its upstream on the first push.
pub fn push(root: &Path) -> AppResult<String> {
    let has_upstream = run(root, &["rev-parse", "--abbrev-ref", "@{u}"]).is_ok();
    if has_upstream { run_all(root, &["push"]) } else { run_all(root, &["push", "--set-upstream", "origin", "HEAD"]) }
}

/// Pulls without merge commits: fast-forward only, like a careful terminal user.
pub fn pull(root: &Path) -> AppResult<String> {
    run_all(root, &["pull", "--ff-only"])
}

pub fn init(root: &Path) -> AppResult<String> {
    run_all(root, &["init"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status() {
        let out = "# branch.oid 1234\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
                   1 .M N... 100644 100644 100644 aaa bbb src/app.ts\0\
                   1 A. N... 000000 100644 100644 000 ccc new file.ts\0\
                   2 R. N... 100644 100644 100644 ddd eee R100 lib/b.ts\0lib/a.ts\0\
                   ? notes.md\0";
        let s = parse_status(out);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        let kinds: Vec<(&str, &str, bool)> = s.changes.iter().map(|c| (c.path.as_str(), c.kind.as_str(), c.staged)).collect();
        assert_eq!(
            kinds,
            [
                ("src/app.ts", "modified", false),
                ("new file.ts", "added", true),
                ("lib/b.ts", "renamed", true),
                ("notes.md", "untracked", false)
            ]
        );
        assert_eq!(s.changes[2].from.as_deref(), Some("lib/a.ts"));
    }

    #[test]
    fn parses_log_and_marks_unpushed() {
        let out = "aaa111\x1faaa\x1fAdam\x1f2026-11-05T10:00:00+07:00\x1fAdd tree\x1e\nbbb222\x1fbbb\x1fAdam\x1f2026-11-04T09:00:00+07:00\x1fFirst\x1e";
        let log = parse_log(out, &["aaa111"]);
        assert_eq!(log.len(), 2);
        assert!(log[0].local && !log[1].local);
        assert_eq!(log[1].subject, "First");
    }

    #[test]
    fn commits_and_logs_a_real_repository() {
        let dir = std::env::temp_dir().join(format!("klikvibe-git-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        assert_eq!(status(&dir).unwrap().changes[0].kind, "untracked");
        assert!(diff(&dir, "a.txt").unwrap().contains("+one"));
        commit(&dir, "First").unwrap();
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        assert!(diff(&dir, "a.txt").unwrap().contains("+two"));
        let log = log(&dir, 10).unwrap();
        assert_eq!(log[0].subject, "First");
        assert!(show(&dir, &log[0].hash).unwrap().contains("a.txt"));
        assert!(show(&dir, "HEAD; rm -rf /").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
