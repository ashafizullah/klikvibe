//! Git through the user's own `git` command line, so their credentials, SSH
//! keys, hooks and config work exactly as in a terminal. Nothing here ever
//! asks for a password: Git is told not to prompt, and a push that needs
//! credentials fails with Git's own message.
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::{Path, PathBuf};
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
    run_env(root, args, &[])
}

/// Like `run`, with extra environment variables (a separate index, a fixed author).
fn run_env(root: &Path, args: &[&str], env: &[(&str, &str)]) -> AppResult<String> {
    let out = git(root).args(args).envs(env.iter().copied()).output().map_err(|e| {
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
    /// Something of it is in the index (to be committed).
    pub staged: bool,
    /// Something of it in the working tree isn't (a file can be both: partly staged).
    pub unstaged: bool,
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
                s.changes.push(Change {
                    path: parts[7].into(),
                    from: None,
                    kind: kind(xy).into(),
                    staged: !xy.starts_with('.'),
                    unstaged: !xy.ends_with('.'),
                });
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // XY sub mH mI mW hH hI Xscore path, then the old path as the next field
            let parts: Vec<&str> = rest.splitn(9, ' ').collect();
            if parts.len() == 9 {
                let from = fields.next().map(String::from);
                s.changes.push(Change {
                    path: parts[8].into(),
                    from,
                    kind: "renamed".into(),
                    staged: !parts[0].starts_with('.'),
                    unstaged: !parts[0].ends_with('.'),
                });
            }
        } else if let Some(rest) = line.strip_prefix("u ") {
            if let Some(path) = rest.splitn(10, ' ').nth(9) {
                s.changes.push(Change { path: path.into(), from: None, kind: "conflict".into(), staged: false, unstaged: true });
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            s.changes.push(Change { path: path.into(), from: None, kind: "untracked".into(), staged: false, unstaged: true });
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
    /// Parent hashes: none for the first commit, two or more for a merge.
    pub parents: Vec<String>,
    /// Refs pointing here, in full as Git decorates them: "HEAD -> refs/heads/main",
    /// "refs/remotes/origin/main", "tag: refs/tags/v0.1.0".
    pub refs: Vec<String>,
}

const LOG_FORMAT: &str = "--pretty=format:%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1f%P%x1f%D%x1e";

pub fn parse_log(out: &str, local: &[&str]) -> Vec<Commit> {
    out.split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches('\n').split('\x1f').collect();
            (f.len() == 7).then(|| Commit {
                local: local.contains(&f[0]),
                hash: f[0].into(),
                short: f[1].into(),
                author: f[2].into(),
                date: f[3].into(),
                subject: f[4].into(),
                parents: f[5].split_whitespace().map(String::from).collect(),
                refs: f[6].split(", ").filter(|r| !r.is_empty()).map(String::from).collect(),
            })
        })
        .collect()
}

/// The newest commits of the current branch, or of every branch and tag with `all`,
/// newest first and children before their parents (for the graph).
pub fn log(root: &Path, limit: u32, all: bool) -> AppResult<Vec<Commit>> {
    // No commits yet: `HEAD` (passed with `all`) would be an unknown revision.
    if run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_err() {
        return Ok(Vec::new());
    }
    let n = format!("-n{}", limit.clamp(1, 1000));
    let mut args = vec!["log", "--date-order", "--decorate=full", &n, LOG_FORMAT];
    if all {
        args.extend(["--branches", "--remotes", "--tags", "HEAD"]);
    }
    let out = match run(root, &args) {
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

fn check_hash(hash: &str) -> AppResult<()> {
    if hash.is_empty() || hash.len() > 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::msg("Invalid commit"));
    }
    Ok(())
}

/// A commit's message, files and patch.
pub fn show(root: &Path, hash: &str) -> AppResult<String> {
    check_hash(hash)?;
    run(root, &["show", "--no-color", "--stat", "--patch", "--format=fuller", hash])
}

/// Git's empty tree: what the first commit is compared with.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// What `hash` is compared with: its first parent (so a merge shows what it brought in), or nothing.
fn base_of(root: &Path, hash: &str) -> String {
    run(root, &["rev-parse", "--verify", "-q", &format!("{hash}^1")]).map(|s| s.trim().to_string()).unwrap_or_else(|_| EMPTY_TREE.into())
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    /// The old path of a rename.
    pub from: Option<String>,
    /// added, modified, deleted or renamed.
    pub kind: String,
    /// Lines added and removed; None for binary files.
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

/// Parses `git diff --name-status -z` and `--numstat -z` (both with -M) into files.
pub fn parse_commit_files(names: &str, nums: &str) -> Vec<CommitFile> {
    let mut files = Vec::new();
    let mut f = names.split('\0').filter(|s| !s.is_empty());
    while let Some(status) = f.next() {
        let kind = match status.chars().next() {
            Some('A') => "added",
            Some('D') => "deleted",
            Some('R') | Some('C') => "renamed",
            _ => "modified",
        };
        let (from, path) = if kind == "renamed" { (f.next().map(String::from), f.next()) } else { (None, f.next()) };
        let Some(path) = path else { break };
        files.push(CommitFile { path: path.into(), from, kind: kind.into(), added: None, removed: None });
    }
    // numstat: "added\tremoved\tpath", or "added\tremoved\t" then old and new paths for a rename.
    let mut n = nums.split('\0');
    while let Some(head) = n.next() {
        let parts: Vec<&str> = head.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let path = if parts[2].is_empty() {
            let _old = n.next();
            n.next().unwrap_or_default()
        } else {
            parts[2]
        };
        if let Some(file) = files.iter_mut().find(|x| x.path == path) {
            file.added = parts[0].parse().ok();
            file.removed = parts[1].parse().ok();
        }
    }
    files
}

/// The files a commit changed (against its first parent).
pub fn commit_files(root: &Path, hash: &str) -> AppResult<Vec<CommitFile>> {
    check_hash(hash)?;
    let base = base_of(root, hash);
    let names = run(root, &["diff", "--no-color", "-M", "-z", "--name-status", &base, hash])?;
    let nums = run(root, &["diff", "--no-color", "-M", "-z", "--numstat", &base, hash])?;
    Ok(parse_commit_files(&names, &nums))
}

/// One file's changes in a commit (`from`: its old path, for a rename).
pub fn commit_file_diff(root: &Path, hash: &str, path: &str, from: Option<&str>) -> AppResult<String> {
    check_hash(hash)?;
    let base = base_of(root, hash);
    let mut args = vec!["--literal-pathspecs", "diff", "--no-color", "-M", &base, hash, "--", path];
    args.extend(from);
    run(root, &args)
}

/// Commits what is staged; with `all`, stages everything first.
pub fn commit(root: &Path, message: &str, all: bool) -> AppResult<String> {
    if message.trim().is_empty() {
        return Err(AppError::msg("Write a commit message first"));
    }
    if all {
        run(root, &["add", "--all"])?;
    }
    run_all(root, &["commit", "-m", message])
}

/// Adds files (paths as Git reports them, taken literally) to the next commit, deletions included.
pub fn stage(root: &Path, paths: &[String]) -> AppResult<()> {
    let mut args = vec!["--literal-pathspecs", "add", "-A", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args)?;
    Ok(())
}

/// Takes files out of the next commit; their changes stay in the working tree.
pub fn unstage(root: &Path, paths: &[String]) -> AppResult<()> {
    let has_head = run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
    // Before the first commit there is nothing to restore the index from.
    let mut args = if has_head {
        vec!["--literal-pathspecs", "restore", "--staged", "--"]
    } else {
        vec!["--literal-pathspecs", "rm", "--cached", "-q", "-r", "--"]
    };
    args.extend(paths.iter().map(String::as_str));
    run(root, &args)?;
    Ok(())
}

/// The URL of the "origin" remote (else the first remote), for web links; None without remotes.
pub fn remote_url(root: &Path) -> Option<String> {
    let name = match run(root, &["remote"]) {
        Ok(list) if list.lines().any(|l| l.trim() == "origin") => "origin".to_string(),
        Ok(list) => list.lines().next()?.trim().to_string(),
        Err(_) => return None,
    };
    run(root, &["remote", "get-url", &name]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
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

/// Throws away the changes to `path` (and to `from`, the old path of a
/// rename): back to the last commit, or gone for a file Git doesn't track.
/// Paths are literal, so a name like `*.ts` never matches other files.
pub fn discard(root: &Path, path: &str, from: Option<&str>, untracked: bool) -> AppResult<()> {
    if untracked {
        // `clean` only ever deletes untracked files.
        run(root, &["--literal-pathspecs", "clean", "-f", "-q", "--", path])?;
        return Ok(());
    }
    let has_head = run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
    let mut args = if has_head {
        // No-overlay mode: files missing from HEAD (added ones) are removed.
        vec!["--literal-pathspecs", "restore", "--source=HEAD", "--staged", "--worktree", "--", path]
    } else {
        // Before the first commit, every change is an added file.
        vec!["--literal-pathspecs", "rm", "-f", "-q", "--", path]
    };
    args.extend(from);
    run(root, &args)?;
    Ok(())
}

/// `rel` as a .gitignore line that matches only it, from the project root.
pub fn ignore_line(rel: &str, dir: bool) -> String {
    let mut line = String::from("/");
    for c in rel.chars() {
        if matches!(c, '*' | '?' | '[' | '\\') {
            line.push('\\');
        }
        line.push(c);
    }
    // Trailing spaces are dropped unless escaped.
    let trimmed = line.trim_end_matches(' ').len();
    let spaces = line.len() - trimmed;
    line.truncate(trimmed);
    line.push_str(&"\\ ".repeat(spaces));
    if dir {
        line.push('/');
    }
    line
}

/// Adds `rel` (a file, or a folder with `dir`) to the root .gitignore at `file`.
pub fn ignore(file: &Path, rel: &str, dir: bool) -> AppResult<()> {
    let mut text =
        std::fs::read_to_string(file).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(String::new()) } else { Err(e) })?;
    let line = ignore_line(rel, dir);
    if text.lines().any(|l| l == line) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line);
    text.push('\n');
    std::fs::write(file, text)?;
    Ok(())
}

// ---- checkpoints ----
//
// Snapshots of the working tree while an agent works, so its changes can be
// undone even before anything is committed. Each is a commit on a ref of its
// own (refs/worktree/klikvibe/checkpoints: per worktree, so a task folder and the
// main folder keep separate ones, and `git push` leaves it alone), built from a
// separate index (.git/klikvibe-index), so the user's index, staging, stash,
// branches and history are never touched. `.gitignore` applies, as for `git add -A`.

const CHECKPOINTS: &str = "refs/worktree/klikvibe/checkpoints";
const CHECKPOINT_AUTHOR: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "KlikVibe"),
    ("GIT_AUTHOR_EMAIL", "checkpoints@klikvibe.local"),
    ("GIT_COMMITTER_NAME", "KlikVibe"),
    ("GIT_COMMITTER_EMAIL", "checkpoints@klikvibe.local"),
];

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub hash: String,
    pub short: String,
    /// ISO 8601.
    pub date: String,
    /// Why it was taken: "opened", "changes", "manual", "agent", "before-restore" or "restored".
    pub reason: String,
    /// A note from whoever took it by hand (the user, or an agent over MCP); may be empty.
    pub note: String,
    /// Files that differ from the checkpoint before it (all files for the first).
    pub files: u32,
}

/// The separate index; checkpoints need the project folder to be the repository's top
/// folder, since their paths are the repository's.
fn checkpoint_index(root: &Path) -> AppResult<String> {
    let top = run(root, &["rev-parse", "--show-toplevel"])
        .map_err(|_| AppError::msg("Checkpoints need Git: this folder isn't a repository yet"))?;
    let same = |a: &Path, b: &Path| a.canonicalize().ok() == b.canonicalize().ok();
    if !same(Path::new(top.trim()), root) {
        return Err(AppError::msg("Checkpoints work when the project folder is the repository's top folder"));
    }
    let git_dir = run(root, &["rev-parse", "--absolute-git-dir"])?;
    Ok(Path::new(git_dir.trim()).join("klikvibe-index").display().to_string())
}

/// Takes a checkpoint of the working tree; None when nothing changed since the last one.
pub fn checkpoint(root: &Path, reason: &str, note: &str) -> AppResult<Option<Checkpoint>> {
    let index = checkpoint_index(root)?;
    let env = [("GIT_INDEX_FILE", index.as_str())];
    run_env(root, &["add", "-A", "--", "."], &env)?;
    let tree = run_env(root, &["write-tree"], &env)?.trim().to_string();
    let parent = run(root, &["rev-parse", "--verify", "-q", CHECKPOINTS]).ok().map(|s| s.trim().to_string());
    let files = match &parent {
        Some(p) => {
            let before = run(root, &["rev-parse", &format!("{p}^{{tree}}")])?.trim().to_string();
            if before == tree {
                return Ok(None);
            }
            run(root, &["diff-tree", "-r", "--name-only", &before, &tree])?.lines().count()
        }
        None => run(root, &["ls-tree", "-r", "--name-only", &tree])?.lines().count(),
    };
    let reason: String = reason.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(40).collect();
    // One line, as a commit message paragraph can't hold more without changing its meaning here.
    let note: String = note.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect();
    let message = if note.is_empty() { format!("{reason}\n\n{files}") } else { format!("{reason}\n\n{files}\n\n{note}") };
    let mut args = vec!["commit-tree", tree.as_str(), "-m", message.as_str()];
    if let Some(p) = &parent {
        args.extend(["-p", p.as_str()]);
    }
    let commit = run_env(root, &args, &CHECKPOINT_AUTHOR)?.trim().to_string();
    // Moves the ref only from where it was read (an empty old value: it must not exist yet).
    run(root, &["update-ref", "-m", "KlikVibe checkpoint", CHECKPOINTS, &commit, parent.as_deref().unwrap_or("")])?;
    Ok(checkpoints(root, 1)?.into_iter().next())
}

/// The newest checkpoints first.
pub fn checkpoints(root: &Path, limit: u32) -> AppResult<Vec<Checkpoint>> {
    if run(root, &["rev-parse", "--verify", "-q", CHECKPOINTS]).is_err() {
        return Ok(Vec::new());
    }
    let n = format!("-n{}", limit.clamp(1, 500));
    let out = run(root, &["log", &n, "--format=%H%x1f%h%x1f%cI%x1f%s%x1f%b%x1e", CHECKPOINTS])?;
    Ok(out
        .split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches('\n').split('\x1f').collect();
            (f.len() == 5).then(|| {
                // The body: the file count, then the note if there is one.
                let mut body = f[4].trim().splitn(2, '\n');
                Checkpoint {
                    hash: f[0].into(),
                    short: f[1].into(),
                    date: f[2].into(),
                    reason: f[3].into(),
                    files: body.next().unwrap_or("").trim().parse().unwrap_or(0),
                    note: body.next().unwrap_or("").trim().to_string(),
                }
            })
        })
        .collect())
}

/// Puts the working tree back as it was at checkpoint `hash`: files changed since
/// are rewritten, files added since are deleted. The state before is saved as a
/// checkpoint first, so this can be undone too. Ignored files are left alone.
pub fn restore_checkpoint(root: &Path, hash: &str) -> AppResult<()> {
    check_hash(hash)?;
    checkpoint(root, "before-restore", "")?;
    let current = run(root, &["rev-parse", "--verify", CHECKPOINTS])?.trim().to_string();
    let changes = run(root, &["diff-tree", "-r", "-z", "--no-renames", "--name-status", hash, &current])?;
    let mut fields = changes.split('\0').filter(|s| !s.is_empty());
    let mut bring_back = Vec::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        if status == "A" {
            // Added since the checkpoint.
            let file = root.join(path);
            if !Path::new(path).components().all(|c| matches!(c, std::path::Component::Normal(_))) || !file.starts_with(root) {
                continue;
            }
            let _ = std::fs::remove_file(&file);
        } else {
            bring_back.push(path.to_string());
        }
    }
    for chunk in bring_back.chunks(200) {
        let mut args = vec!["--literal-pathspecs", "restore", "--source", hash, "--worktree", "--"];
        args.extend(chunk.iter().map(String::as_str));
        run(root, &args)?;
    }
    checkpoint(root, "restored", "")?;
    Ok(())
}

/// Forgets every checkpoint (Git can then reclaim their space).
pub fn clear_checkpoints(root: &Path) -> AppResult<()> {
    let index = checkpoint_index(root)?;
    if run(root, &["rev-parse", "--verify", "-q", CHECKPOINTS]).is_ok() {
        run(root, &["update-ref", "-d", CHECKPOINTS])?;
    }
    let _ = std::fs::remove_file(index);
    Ok(())
}

// ---- worktrees: parallel tasks ----
//
// A task gets its own branch and its own folder next to the repository
// (<repo>.worktrees/<repo>-<task>), opened as a project of its own so an
// agent can work there while another works in the main folder.

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    /// None when detached.
    pub branch: Option<String>,
    /// The repository's main folder (not a linked worktree).
    pub main: bool,
}

/// Parses `git worktree list --porcelain`.
pub fn parse_worktrees(out: &str) -> Vec<Worktree> {
    let mut list: Vec<Worktree> = Vec::new();
    for line in out.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            let main = list.is_empty();
            list.push(Worktree { path: path.into(), branch: None, main });
        } else if let (Some(branch), Some(last)) = (line.strip_prefix("branch "), list.last_mut()) {
            last.branch = Some(branch.trim_start_matches("refs/heads/").into());
        }
    }
    list
}

pub fn worktrees(root: &Path) -> AppResult<Vec<Worktree>> {
    Ok(parse_worktrees(&run(root, &["worktree", "list", "--porcelain"])?))
}

/// A task name as a branch and folder name: lower case, letters, digits and dashes.
pub fn task_slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').chars().take(48).collect()
}

/// Makes a worktree for a new task on a new branch from the current commit; returns its folder.
pub fn add_worktree(root: &Path, name: &str) -> AppResult<PathBuf> {
    let slug = task_slug(name);
    if slug.is_empty() {
        return Err(AppError::msg("Give the task a name with letters or digits"));
    }
    let main = worktrees(root)?.into_iter().find(|w| w.main).ok_or_else(|| AppError::msg("No repository here"))?;
    let main = PathBuf::from(main.path);
    let repo = main.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "repo".into());
    let parent = main.parent().ok_or_else(|| AppError::msg("The repository has no parent folder"))?;
    let dir = parent.join(format!("{repo}.worktrees")).join(format!("{repo}-{slug}"));
    if dir.exists() {
        return Err(AppError::msg(format!("{} already exists", dir.display())));
    }
    let dir_s = dir.display().to_string();
    run_all(root, &["worktree", "add", "-b", &slug, &dir_s])?;
    Ok(dir)
}

/// The repository's main folder, for merging into it and removing worktrees from it.
fn main_worktree(root: &Path) -> AppResult<PathBuf> {
    worktrees(root)?.into_iter().find(|w| w.main).map(|w| PathBuf::from(w.path)).ok_or_else(|| AppError::msg("No repository here"))
}

/// Merges a task's branch into whatever the main folder has checked out (a merge commit, so the task stays visible).
pub fn merge_task(root: &Path, branch: &str) -> AppResult<String> {
    if branch.is_empty() || branch.starts_with('-') || run(root, &["check-ref-format", "--branch", branch]).is_err() {
        return Err(AppError::msg("Invalid branch"));
    }
    run_all(&main_worktree(root)?, &["merge", "--no-ff", "--no-edit", branch])
}

/// Removes a task's worktree folder (`force`: even with uncommitted changes); the branch stays.
pub fn remove_worktree(root: &Path, path: &str, force: bool) -> AppResult<()> {
    let known = worktrees(root)?;
    if !known.iter().any(|w| !w.main && w.path == path) {
        return Err(AppError::msg("That folder isn't one of this repository's task worktrees"));
    }
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path);
    run_all(&main_worktree(root)?, &args)?;
    Ok(())
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
        assert_eq!(s.changes.iter().map(|c| c.unstaged).collect::<Vec<_>>(), [true, false, false, true]);
        assert_eq!(s.changes[2].from.as_deref(), Some("lib/a.ts"));
    }

    #[test]
    fn parses_log_and_marks_unpushed() {
        let out = "aaa111\x1faaa\x1fAdam\x1f2026-11-05T10:00:00+07:00\x1fMerge feature\x1fbbb222 ccc333\x1fHEAD -> refs/heads/main, refs/remotes/origin/main, tag: refs/tags/v0.2.0\x1e\n\
                   bbb222\x1fbbb\x1fAdam\x1f2026-11-04T09:00:00+07:00\x1fFirst\x1f\x1f\x1e";
        let log = parse_log(out, &["aaa111"]);
        assert_eq!(log.len(), 2);
        assert!(log[0].local && !log[1].local);
        assert_eq!(log[1].subject, "First");
        assert_eq!(log[0].parents, ["bbb222", "ccc333"]);
        assert_eq!(log[0].refs, ["HEAD -> refs/heads/main", "refs/remotes/origin/main", "tag: refs/tags/v0.2.0"]);
        assert!(log[1].parents.is_empty() && log[1].refs.is_empty());
    }

    #[test]
    fn commits_and_logs_a_real_repository() {
        let dir = std::env::temp_dir().join(format!("klikvibe-git-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        // A repository without commits has an empty history, on one branch or all.
        assert!(log(&dir, 10, true).unwrap().is_empty());
        assert!(log(&dir, 10, false).unwrap().is_empty());
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        assert_eq!(status(&dir).unwrap().changes[0].kind, "untracked");
        assert!(diff(&dir, "a.txt").unwrap().contains("+one"));
        commit(&dir, "First", true).unwrap();
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        assert!(diff(&dir, "a.txt").unwrap().contains("+two"));
        let log = log(&dir, 10, true).unwrap();
        assert_eq!(log[0].subject, "First");
        assert!(show(&dir, &log[0].hash).unwrap().contains("a.txt"));
        assert!(show(&dir, "HEAD; rm -rf /").is_err());
        // The first commit's files, against nothing; then a change.
        let first = &log[0].hash;
        assert_eq!(commit_files(&dir, first).unwrap()[0].kind, "added");
        std::fs::write(dir.join("b.txt"), "x\ny\n").unwrap();
        commit(&dir, "Second", true).unwrap();
        let second = log_head(&dir);
        let files = commit_files(&dir, &second).unwrap();
        let paths: Vec<(&str, &str, Option<u32>)> = files.iter().map(|f| (f.path.as_str(), f.kind.as_str(), f.added)).collect();
        assert_eq!(paths, [("a.txt", "modified", Some(1)), ("b.txt", "added", Some(2))]);
        assert!(commit_file_diff(&dir, &second, "a.txt", None).unwrap().contains("+two"));
        assert!(commit_files(&dir, "HEAD").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn log_head(dir: &Path) -> String {
        run(dir, &["rev-parse", "HEAD"]).unwrap().trim().to_string()
    }

    #[test]
    fn parses_renames_and_binaries_in_a_commit() {
        let names = "M\0src/a.ts\0R100\0old.ts\0new.ts\0A\0logo.png\0";
        let nums = "3\t1\tsrc/a.ts\0".to_string() + "0\t0\t\0old.ts\0new.ts\0" + "-\t-\tlogo.png\0";
        let files = parse_commit_files(names, &nums);
        assert_eq!(files.len(), 3);
        assert_eq!((files[0].added, files[0].removed), (Some(3), Some(1)));
        assert_eq!(files[1].from.as_deref(), Some("old.ts"));
        assert_eq!((files[1].kind.as_str(), files[1].path.as_str()), ("renamed", "new.ts"));
        assert_eq!(files[2].added, None);
    }

    #[test]
    fn stages_and_commits_only_what_is_staged() {
        let dir = std::env::temp_dir().join(format!("klikvibe-stage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        // Before the first commit, unstaging works too.
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        stage(&dir, &["a.txt".into()]).unwrap();
        unstage(&dir, &["a.txt".into()]).unwrap();
        assert!(!status(&dir).unwrap().changes[0].staged);
        std::fs::write(dir.join("*.txt"), "glob-looking name\n").unwrap();
        commit(&dir, "First", true).unwrap();

        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        std::fs::write(dir.join("b.txt"), "new\n").unwrap();
        // "*.txt" is a file name here, not a pattern: a.txt stays unstaged.
        std::fs::write(dir.join("*.txt"), "changed\n").unwrap();
        stage(&dir, &["b.txt".into(), "*.txt".into()]).unwrap();
        let st = status(&dir).unwrap();
        let staged: Vec<&str> = st.changes.iter().filter(|c| c.staged).map(|c| c.path.as_str()).collect();
        assert_eq!(staged.len(), 2);
        assert!(!st.changes.iter().find(|c| c.path == "a.txt").unwrap().staged);

        commit(&dir, "Only staged", false).unwrap();
        let left = status(&dir).unwrap().changes;
        assert_eq!(left.len(), 1);
        assert_eq!((left[0].path.as_str(), left[0].staged, left[0].unstaged), ("a.txt", false, true));
        assert!(commit(&dir, "Nothing staged", false).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn discards_literally() {
        let dir = std::env::temp_dir().join(format!("klikvibe-discard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        // Before the first commit: an added file goes away.
        std::fs::write(dir.join("first.txt"), "1\n").unwrap();
        run(&dir, &["add", "first.txt"]).unwrap();
        discard(&dir, "first.txt", None, false).unwrap();
        assert!(!dir.join("first.txt").exists());
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        std::fs::write(dir.join("b.txt"), "one\n").unwrap();
        commit(&dir, "First", true).unwrap();
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        std::fs::write(dir.join("b.txt"), "two\n").unwrap();
        // A glob-looking name matches nothing, and leaves the others alone.
        assert!(discard(&dir, "*.txt", None, false).is_err());
        discard(&dir, "a.txt", None, false).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "one\n");
        assert_eq!(std::fs::read_to_string(dir.join("b.txt")).unwrap(), "two\n");
        // A rename goes back to the old name.
        run(&dir, &["mv", "b.txt", "c.txt"]).unwrap();
        discard(&dir, "c.txt", Some("b.txt"), false).unwrap();
        assert!(dir.join("b.txt").exists() && !dir.join("c.txt").exists());
        // Untracked files are deleted; `clean` never touches tracked ones.
        std::fs::write(dir.join("new.log"), "x").unwrap();
        discard(&dir, "new.log", None, true).unwrap();
        assert!(!dir.join("new.log").exists());
        discard(&dir, "a.txt", None, true).unwrap();
        assert!(dir.join("a.txt").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn checkpoints_snapshot_and_restore_without_touching_the_index() {
        let dir = std::env::temp_dir().join(format!("klikvibe-checkpoint-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        std::fs::write(dir.join(".gitignore"), "build/\n").unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        commit(&dir, "First", true).unwrap();
        // The user's own staged change must survive everything below.
        std::fs::write(dir.join("staged.txt"), "staged\n").unwrap();
        run(&dir, &["add", "staged.txt"]).unwrap();

        let base = checkpoint(&dir, "opened", "").unwrap().unwrap();
        assert_eq!((base.reason.as_str(), base.note.as_str()), ("opened", ""));
        assert!(checkpoint(&dir, "changes", "").unwrap().is_none(), "nothing changed");

        // An agent edits, adds and deletes; an ignored build folder appears.
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        std::fs::write(dir.join("new.txt"), "new\n").unwrap();
        std::fs::create_dir_all(dir.join("build")).unwrap();
        std::fs::write(dir.join("build/out.js"), "x").unwrap();
        let after = checkpoint(&dir, "agent", "before the\nrefactor").unwrap().unwrap();
        assert_eq!(after.files, 2);
        assert_eq!((after.reason.as_str(), after.note.as_str()), ("agent", "before the refactor"));
        assert_eq!(checkpoints(&dir, 10).unwrap().len(), 2);

        restore_checkpoint(&dir, &base.hash).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "one\n");
        assert!(!dir.join("new.txt").exists());
        assert!(dir.join("build/out.js").exists(), "ignored files are left alone");
        // Nothing had changed since "changes", so no before-restore copy; the restore can be undone from there.
        let list = checkpoints(&dir, 10).unwrap();
        assert_eq!(list.iter().map(|c| c.reason.as_str()).collect::<Vec<_>>(), ["restored", "agent", "opened"]);
        restore_checkpoint(&dir, &list[1].hash).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two\n");
        assert!(dir.join("new.txt").exists());
        // Edits after the last checkpoint are saved before a restore.
        std::fs::write(dir.join("a.txt"), "three\n").unwrap();
        restore_checkpoint(&dir, &base.hash).unwrap();
        assert_eq!(checkpoints(&dir, 1).unwrap()[0].reason, "restored");
        assert_eq!(checkpoints(&dir, 10).unwrap()[1].reason, "before-restore");

        // HEAD, the index and the branch list are as the user left them.
        assert_eq!(log(&dir, 10, true).unwrap().len(), 1);
        assert!(status(&dir).unwrap().changes.iter().any(|c| c.path == "staged.txt" && c.staged));
        assert!(run(&dir, &["branch", "--list"]).unwrap().lines().count() == 1);

        clear_checkpoints(&dir).unwrap();
        assert!(checkpoints(&dir, 10).unwrap().is_empty());
        assert!(restore_checkpoint(&dir, "not-a-hash").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tasks_in_worktrees_merge_back() {
        let base = std::env::temp_dir().join(format!("klikvibe-wt-{}", uuid::Uuid::new_v4()));
        let dir = base.join("app");
        std::fs::create_dir_all(&dir).unwrap();
        init(&dir).unwrap();
        run(&dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(&dir, &["config", "user.name", "Test"]).unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        commit(&dir, "First", true).unwrap();

        assert_eq!(task_slug("  Fix the Login page! "), "fix-the-login-page");
        let wt = add_worktree(&dir, "Fix login").unwrap();
        assert!(wt.ends_with("app.worktrees/app-fix-login"));
        let list = worktrees(&dir).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].main && !list[1].main);
        assert_eq!(list[1].branch.as_deref(), Some("fix-login"));
        assert!(add_worktree(&dir, "fix login").is_err(), "the folder exists");

        // The task commits in its own folder, then merges into the main one.
        std::fs::write(wt.join("b.txt"), "task\n").unwrap();
        commit(&wt, "Task work", true).unwrap();
        assert!(!dir.join("b.txt").exists());
        merge_task(&wt, "fix-login").unwrap();
        assert!(dir.join("b.txt").exists());
        assert!(merge_task(&dir, "--evil").is_err());

        // Each folder keeps its own checkpoints (refs under refs/worktree/ aren't shared).
        checkpoint(&dir, "opened", "").unwrap().unwrap();
        checkpoint(&wt, "opened", "").unwrap().unwrap();
        assert_eq!(checkpoints(&dir, 10).unwrap().len(), 1);
        assert_eq!(checkpoints(&wt, 10).unwrap().len(), 1);

        // Uncommitted changes keep the folder unless forced.
        std::fs::write(wt.join("c.txt"), "draft\n").unwrap();
        let wt_s = list[1].path.clone();
        assert!(remove_worktree(&dir, &wt_s, false).is_err());
        remove_worktree(&dir, &wt_s, true).unwrap();
        assert!(!wt.exists());
        assert!(remove_worktree(&dir, &dir.display().to_string(), true).is_err(), "never the main folder");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn ignore_lines_match_one_path() {
        assert_eq!(ignore_line("dist/app.js", false), "/dist/app.js");
        assert_eq!(ignore_line("tmp", true), "/tmp/");
        assert_eq!(ignore_line("a*b?[c].txt", false), "/a\\*b\\?\\[c].txt");
        assert_eq!(ignore_line("#notes ", false), "/#notes\\ ");
        let dir = std::env::temp_dir().join(format!("klikvibe-ignore-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(".gitignore");
        std::fs::write(&file, "node_modules").unwrap();
        ignore(&file, "out.log", false).unwrap();
        ignore(&file, "out.log", false).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "node_modules\n/out.log\n");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
