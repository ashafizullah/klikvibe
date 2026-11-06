//! Listing, reading and writing files inside the open project.
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Files larger than this open read-only as "too large".
const MAX_TEXT: u64 = 5 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    /// Left out by `.gitignore` (shown dimmed, like other editors do).
    pub ignored: bool,
}

/// The folder's entries, `.git` itself left out, folders first, then by name.
/// What Git ignores is listed too, marked `ignored`.
pub fn list(dir: &Path) -> AppResult<Vec<Entry>> {
    let kept: HashSet<PathBuf> = ignore::WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.depth() == 1)
        .map(ignore::DirEntry::into_path)
        .collect();
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != ".git")
        .map(|e| {
            let path = e.path();
            Entry {
                name: e.file_name().to_string_lossy().into_owned(),
                dir: e.file_type().is_ok_and(|t| t.is_dir()),
                ignored: !kept.contains(&path),
                path: path.display().to_string(),
            }
        })
        .collect();
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileText {
    /// The text, or None for binary or too-large files.
    pub text: Option<String>,
    pub size: u64,
    pub binary: bool,
    pub too_large: bool,
}

pub fn read(path: &Path) -> AppResult<FileText> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_TEXT {
        return Ok(FileText { text: None, size, binary: false, too_large: true });
    }
    let mut bytes = Vec::with_capacity(size as usize);
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;
    // A NUL byte early on means a binary file, as Git decides it.
    let binary = bytes.iter().take(8000).any(|&b| b == 0);
    let text = if binary { None } else { String::from_utf8(bytes).ok() };
    Ok(FileText { binary: binary || text.is_none(), text, size, too_large: false })
}

/// Most files listed for name search; more than this is cut off (and said so).
const MAX_FILES: usize = 50_000;
/// Content search: files larger than this are skipped, and it stops after this many matches.
const MAX_SEARCH_FILE: u64 = 1024 * 1024;
const MAX_MATCHES: usize = 2_000;
/// A matching line is cut to about this many characters around the match.
const LINE_CONTEXT: usize = 160;

/// Every file in the project the way Git sees it (`.gitignore` applied), as
/// paths relative to the root with forward slashes, for search by name.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    pub files: Vec<String>,
    pub truncated: bool,
}

fn walk(root: &Path, include: Option<&str>) -> AppResult<ignore::Walk> {
    let mut b = ignore::WalkBuilder::new(root);
    b.hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git");
    if let Some(globs) = include.map(str::trim).filter(|g| !g.is_empty()) {
        // "*.ts, src/**/*.svelte": gitignore-style globs, any of them.
        let mut o = ignore::overrides::OverrideBuilder::new(root);
        for g in globs.split([',', ' ']).filter(|g| !g.is_empty()) {
            o.add(g).map_err(|e| AppError::msg(e.to_string()))?;
        }
        b.overrides(o.build().map_err(|e| AppError::msg(e.to_string()))?);
    }
    Ok(b.build())
}

fn rel(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
}

pub fn list_all(root: &Path) -> AppResult<FileList> {
    let mut files = Vec::new();
    let mut truncated = false;
    for entry in walk(root, None)?.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if files.len() >= MAX_FILES {
            truncated = true;
            break;
        }
        if let Some(r) = rel(root, entry.path()) {
            files.push(r);
        }
    }
    files.sort();
    Ok(FileList { files, truncated })
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextMatch {
    /// Relative to the root, forward slashes.
    pub path: String,
    /// 1-based.
    pub line: u32,
    /// The line (trimmed, cut around the match) and the match's byte range in it.
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSearch {
    pub matches: Vec<TextMatch>,
    pub truncated: bool,
}

/// Lines containing `query` in the project's text files, optionally only in
/// files matching `include`. Smart case: a query with a capital letter is
/// matched exactly, otherwise ASCII letters match either case.
pub fn search_text(root: &Path, query: &str, include: Option<&str>) -> AppResult<TextSearch> {
    let mut matches = Vec::new();
    if query.is_empty() {
        return Ok(TextSearch { matches, truncated: false });
    }
    let exact = query.chars().any(char::is_uppercase);
    let needle = if exact { query.to_string() } else { query.to_ascii_lowercase() };
    for entry in walk(root, include)?.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|t| t.is_file()) || entry.metadata().map_or(true, |m| m.len() > MAX_SEARCH_FILE) {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        if bytes.iter().take(8000).any(|&b| b == 0) {
            continue;
        }
        let Some(path) = rel(root, entry.path()) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        for (i, line) in text.lines().enumerate() {
            // ASCII lowercasing keeps byte offsets, so the match maps back onto the line.
            let hay = if exact { std::borrow::Cow::Borrowed(line) } else { std::borrow::Cow::Owned(line.to_ascii_lowercase()) };
            let Some(at) = hay.find(&needle) else { continue };
            if matches.len() >= MAX_MATCHES {
                return Ok(TextSearch { matches, truncated: true });
            }
            matches.push(snippet(&path, i + 1, line, at, at + needle.len()));
        }
    }
    Ok(TextSearch { matches, truncated: false })
}

// ---- go to definition, the light way ----
//
// No language server: lines that look like they define `name` in the common
// languages (function, class, struct, def, fn, const, a method, ...). Good for
// most jumps; a name defined in several places gives several results to pick from.

/// At most this many definitions are returned.
const MAX_DEFINITIONS: usize = 50;

fn definition_patterns(name: &str) -> AppResult<regex::Regex> {
    let n = regex::escape(name);
    let modifiers = r#"(?:(?:export|default|pub(?:\([^)]*\))?|async|unsafe|extern(?:\s+"[^"]*")?|static|abstract|declare|public|private|protected|internal|open|final|sealed|data|override|inline|readonly|local)\s+)*"#;
    let keyword = r"(?:function\*?|class|interface|type|enum|struct|union|trait|fn|def|func|const|let|var|static|mod|module|namespace|record|object|val|typealias|macro_rules!|protocol|extension)";
    let patterns = [
        // function x / class X / fn x / def x / const x / func (r *T) x / def self.x
        format!(r"^\s*{modifiers}{keyword}\s+(?:\([^)]*\)\s*)?(?:self\.)?{n}\b"),
        // A method: x(a, b) {  /  async x(): T {  /  public void x(int a) {
        format!(
            r"^\s*(?:(?:async|static|public|private|protected|get|set|override|readonly|final|virtual)\s+)*(?:[\w<>\[\]?,.]+\s+)?{n}\s*(?:<[^>]*>)?\s*\([^)]*\)\s*(?::\s*[^{{=;]+)?\s*\{{"
        ),
        // An object's member: x: function / x: (a) => / x: async (a) =>
        format!(r"^\s*{n}\s*:\s*(?:async\s+)?(?:function\b|\([^)]*\)\s*=>|\w+\s*=>)"),
        // A top-level assignment (Python, shell): X = ...
        format!(r"^{n}\s*(?::\s*[^=]+)?=(?:[^=]|$)"),
    ];
    regex::Regex::new(&patterns.join("|")).map_err(|e| AppError::msg(e.to_string()))
}

/// Lines in the project's text files that look like they define `name`.
pub fn find_definitions(root: &Path, name: &str) -> AppResult<Vec<TextMatch>> {
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        return Ok(Vec::new());
    }
    let re = definition_patterns(name)?;
    // Not a word that merely contains the name: "update" for "date".
    let keywords = ["if", "for", "while", "switch", "catch", "return", "new", "await", "else", "function"];
    let mut found = Vec::new();
    for entry in walk(root, None)?.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|t| t.is_file()) || entry.metadata().map_or(true, |m| m.len() > MAX_SEARCH_FILE) {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        if bytes.iter().take(8000).any(|&b| b == 0) {
            continue;
        }
        let Some(path) = rel(root, entry.path()) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        for (i, line) in text.lines().enumerate() {
            if !line.contains(name) || keywords.contains(&name) {
                continue;
            }
            if !re.is_match(line) {
                continue;
            }
            let at = word_at(line, name).unwrap_or(0);
            found.push(snippet(&path, i + 1, line, at, at + name.len()));
            if found.len() >= MAX_DEFINITIONS {
                return Ok(found);
            }
        }
    }
    Ok(found)
}

/// Where `name` stands as a whole word in `line`.
fn word_at(line: &str, name: &str) -> Option<usize> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    line.match_indices(name)
        .map(|(i, _)| i)
        .find(|&i| !line[..i].chars().next_back().is_some_and(is_word) && !line[i + name.len()..].chars().next().is_some_and(is_word))
}

/// The line around a match, trimmed and cut to LINE_CONTEXT characters, with the match's new range.
fn snippet(path: &str, line_no: usize, line: &str, start: usize, end: usize) -> TextMatch {
    let lead = line.len() - line.trim_start().len();
    let mut from = lead.min(start);
    if end - from > LINE_CONTEXT {
        from = start.saturating_sub(LINE_CONTEXT / 4).max(lead);
    }
    while !line.is_char_boundary(from) {
        from -= 1;
    }
    let mut to = line.trim_end().len().max(end).min(from + LINE_CONTEXT.max(end - from));
    while !line.is_char_boundary(to) {
        to += 1;
    }
    let prefix = if from > lead { "…" } else { "" };
    let text = format!("{prefix}{}", &line[from..to]);
    let shift = prefix.len() as isize - from as isize;
    TextMatch {
        path: path.into(),
        line: line_no as u32,
        start: (start as isize + shift) as usize,
        end: (end as isize + shift) as usize,
        text,
    }
}

/// Makes a new empty file (with the folders on the way) or a new folder; `path` comes from Projects::fresh.
pub fn create(path: &Path, folder: bool) -> AppResult<()> {
    if folder {
        std::fs::create_dir_all(path)?;
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    }
    Ok(())
}

/// A name for renaming: one plain file or folder name.
pub fn plain_name(name: &str) -> AppResult<&str> {
    let n = name.trim();
    let mut parts = Path::new(n).components();
    match (parts.next(), parts.next()) {
        (Some(std::path::Component::Normal(c)), None) if c != ".git" => Ok(n),
        _ => Err(crate::error::AppError::msg("Use a plain name, without / or ..")),
    }
}

pub fn write(path: &Path, text: &str) -> AppResult<()> {
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_across_languages() {
        let dir = std::env::temp_dir().join(format!("klikvibe-defs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        file(
            "a.ts",
            "import { save } from './b';\nexport async function save(x: number): Promise<void> {}\nconst result = save(1);\nclass Store {\n  save(item: Item): void {\n  }\n}\nexport const saveAll = () => {};\n",
        );
        file("b.rs", "pub(crate) fn save(root: &Path) -> AppResult<()> {\n    save(root)\n}\npub struct Saver;\n");
        file("c.py", "def save(self):\n    return save()\nSAVE_LIMIT = 10\n");
        file("d.go", "func (s *Store) save() error {\n\treturn nil\n}\n");
        let lines = |name: &str| {
            let mut v: Vec<(String, u32)> = find_definitions(&dir, name).unwrap().into_iter().map(|m| (m.path, m.line)).collect();
            v.sort();
            v
        };
        let at = |p: &str, l: u32| (p.to_string(), l);
        assert_eq!(lines("save"), [at("a.ts", 2), at("a.ts", 5), at("b.rs", 1), at("c.py", 1), at("d.go", 1)]);
        assert_eq!(lines("Saver"), [at("b.rs", 4)]);
        assert_eq!(lines("SAVE_LIMIT"), [at("c.py", 3)]);
        assert_eq!(lines("saveAll"), [at("a.ts", 8)]);
        assert!(lines("result").len() == 1, "a const counts");
        assert!(lines("x.y").is_empty() && lines("").is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lists_like_git_and_reads_text_only() {
        let dir = std::env::temp_dir().join(format!("klikvibe-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join(".gitignore"), "node_modules\n").unwrap();
        std::fs::write(dir.join("README.md"), "# hi\n").unwrap();
        std::fs::write(dir.join("logo.png"), [0x89, b'P', b'N', b'G', 0, 0, 1]).unwrap();
        let entries: Vec<(String, bool)> = list(&dir).unwrap().into_iter().map(|e| (e.name, e.ignored)).collect();
        let want = [("node_modules", true), ("src", false), (".gitignore", false), ("logo.png", false), ("README.md", false)];
        assert_eq!(entries, want.map(|(n, i)| (n.to_string(), i)));
        assert_eq!(read(&dir.join("README.md")).unwrap().text.as_deref(), Some("# hi\n"));
        let png = read(&dir.join("logo.png")).unwrap();
        assert!(png.binary && png.text.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lists_and_searches_like_git() {
        let dir = std::env::temp_dir().join(format!("klikvibe-search-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(dir.join("src/lib")).unwrap();
        std::fs::write(dir.join(".gitignore"), "node_modules\n").unwrap();
        std::fs::write(dir.join("node_modules/x/index.js"), "const Needle = 1;\n").unwrap();
        std::fs::write(dir.join("src/app.ts"), "import x from 'y';\n    const needle = 2; // Needle\n").unwrap();
        std::fs::write(dir.join("src/lib/util.rs"), "fn needle() {}\n").unwrap();
        std::fs::write(dir.join("logo.png"), [0x89, b'P', 0, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();

        let list = list_all(&dir).unwrap();
        assert_eq!(list.files, [".gitignore", "logo.png", "src/app.ts", "src/lib/util.rs"]);

        // Lower case matches either case; binaries and ignored files are skipped.
        let all = search_text(&dir, "needle", None).unwrap();
        let hits: Vec<(&str, u32)> = all.matches.iter().map(|m| (m.path.as_str(), m.line)).collect();
        assert_eq!(hits, [("src/app.ts", 2), ("src/lib/util.rs", 1)]);
        let m = &all.matches[0];
        assert_eq!(m.text, "const needle = 2; // Needle");
        assert_eq!(&m.text[m.start..m.end], "needle");
        // A capital letter makes it exact.
        let exact = search_text(&dir, "Needle", None).unwrap();
        assert_eq!(exact.matches.len(), 1);
        assert_eq!(&exact.matches[0].text[exact.matches[0].start..exact.matches[0].end], "Needle");
        // Only files matching the globs.
        let rs = search_text(&dir, "needle", Some("*.rs")).unwrap();
        assert_eq!(rs.matches.len(), 1);
        assert_eq!(rs.matches[0].path, "src/lib/util.rs");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn long_lines_are_cut_around_the_match() {
        let line = format!("{}target{}", "a".repeat(500), "b".repeat(500));
        let m = snippet("x", 1, &line, 500, 506);
        assert!(m.text.starts_with('…') && m.text.len() < 200);
        assert_eq!(&m.text[m.start..m.end], "target");
    }
}
