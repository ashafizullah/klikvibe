//! Listing, reading and writing files inside the open project.
use crate::error::AppResult;
use serde::Serialize;
use std::io::Read;
use std::path::Path;

/// Files larger than this open read-only as "too large".
const MAX_TEXT: u64 = 5 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
}

/// The folder's entries as Git sees them: `.gitignore`d files and `.git`
/// itself left out, folders first, then by name.
pub fn list(dir: &Path) -> AppResult<Vec<Entry>> {
    let mut entries: Vec<Entry> = ignore::WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.depth() == 1)
        .map(|e| Entry {
            name: e.file_name().to_string_lossy().into_owned(),
            path: e.path().display().to_string(),
            dir: e.file_type().is_some_and(|t| t.is_dir()),
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

pub fn write(path: &Path, text: &str) -> AppResult<()> {
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_like_git_and_reads_text_only() {
        let dir = std::env::temp_dir().join(format!("klikvibe-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join(".gitignore"), "node_modules\n").unwrap();
        std::fs::write(dir.join("README.md"), "# hi\n").unwrap();
        std::fs::write(dir.join("logo.png"), [0x89, b'P', b'N', b'G', 0, 0, 1]).unwrap();
        let names: Vec<String> = list(&dir).unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["src", ".gitignore", "logo.png", "README.md"]);
        assert_eq!(read(&dir.join("README.md")).unwrap().text.as_deref(), Some("# hi\n"));
        let png = read(&dir.join("logo.png")).unwrap();
        assert!(png.binary && png.text.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
