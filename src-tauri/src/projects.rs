//! Which folders the window may touch. A folder becomes a project only when
//! the user picks it in the native dialog, opens it from the launch
//! arguments (`klikvibe .`), or reopens one of those from the recent list,
//! which is kept here, not in the webview. Every file and Git command checks
//! its path against these roots, symlinks resolved, so content shown in the
//! window can't reach the rest of the disk.
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const MAX_RECENT: usize = 16;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub root: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub root: String,
    pub name: String,
    pub exists: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    recent: Vec<String>,
}

pub struct Projects {
    /// Canonical roots opened in this run.
    open: Mutex<Vec<PathBuf>>,
    recent: Mutex<Vec<String>>,
    file: PathBuf,
    /// A folder from the launch arguments, until the window takes it.
    pub launch: Mutex<Option<Project>>,
}

fn name_of(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string())
}

impl Projects {
    pub fn new(data_dir: PathBuf) -> Self {
        let file = data_dir.join("recent.json");
        let stored: Stored = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { open: Mutex::new(Vec::new()), recent: Mutex::new(stored.recent), file, launch: Mutex::new(None) }
    }

    fn save(&self, recent: &[String]) {
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&Stored { recent: recent.to_vec() }) {
            let _ = std::fs::write(&self.file, json);
        }
    }

    /// Opens `dir` as a project (the caller vouches the user chose it).
    pub fn admit(&self, dir: &Path) -> AppResult<Project> {
        let root = dunce(dir.canonicalize()?);
        if !root.is_dir() {
            return Err(AppError::msg("Not a folder"));
        }
        let mut open = self.open.lock().unwrap();
        if !open.contains(&root) {
            open.push(root.clone());
        }
        let shown = root.display().to_string();
        let mut recent = self.recent.lock().unwrap();
        recent.retain(|r| r != &shown);
        recent.insert(0, shown.clone());
        recent.truncate(MAX_RECENT);
        self.save(&recent);
        Ok(Project { name: name_of(&root), root: shown })
    }

    /// Reopens a folder from the recent list; anything else is refused.
    pub fn reopen(&self, root: &str) -> AppResult<Project> {
        if !self.recent.lock().unwrap().iter().any(|r| r == root) {
            return Err(AppError::msg("Open this folder with Open Folder first"));
        }
        self.admit(Path::new(root))
    }

    pub fn recent(&self) -> Vec<RecentProject> {
        self.recent
            .lock()
            .unwrap()
            .iter()
            .map(|r| {
                let p = Path::new(r);
                RecentProject { root: r.clone(), name: name_of(p), exists: p.is_dir() }
            })
            .collect()
    }

    pub fn forget(&self, root: &str) {
        let mut recent = self.recent.lock().unwrap();
        recent.retain(|r| r != root);
        self.save(&recent);
    }

    /// The open project root that is exactly `root`.
    pub fn root(&self, root: &str) -> AppResult<PathBuf> {
        let wanted = dunce(Path::new(root).canonicalize().map_err(|_| AppError::msg("Project folder not found"))?);
        self.open.lock().unwrap().iter().find(|r| **r == wanted).cloned().ok_or_else(|| AppError::msg("Project is not open"))
    }

    /// `path` inside an open project, resolved; it must exist.
    pub fn existing(&self, path: &str) -> AppResult<PathBuf> {
        let resolved = dunce(Path::new(path).canonicalize().map_err(|_| AppError::msg(format!("Not found: {path}")))?);
        self.inside(&resolved)?;
        Ok(resolved)
    }

    /// `path` itself inside an open project, not what it points to when it is a symlink:
    /// for renaming and deleting, which act on the link. Its folder is resolved; it must exist.
    pub fn entry(&self, path: &str) -> AppResult<PathBuf> {
        let p = Path::new(path);
        let (Some(parent), Some(name)) = (p.parent(), p.file_name()) else {
            return Err(AppError::msg("Invalid path"));
        };
        let parent = dunce(parent.canonicalize().map_err(|_| AppError::msg(format!("Not found: {path}")))?);
        let at = parent.join(name);
        at.symlink_metadata().map_err(|_| AppError::msg(format!("Not found: {path}")))?;
        self.inside(&at)?;
        Ok(at)
    }

    /// `path` inside an open project for writing: its folder must exist and
    /// be inside, and it must not point into `.git`.
    pub fn writable(&self, path: &str) -> AppResult<PathBuf> {
        let p = Path::new(path);
        if p.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(AppError::msg("Invalid path"));
        }
        let (parent, name) = match (p.parent(), p.file_name()) {
            (Some(parent), Some(name)) => (parent, name),
            _ => return Err(AppError::msg("Invalid path")),
        };
        let parent = dunce(parent.canonicalize().map_err(|_| AppError::msg("Folder not found"))?);
        let target = parent.join(name);
        // An existing symlink is followed to where it really points.
        let target = match target.canonicalize() {
            Ok(real) => dunce(real),
            Err(_) => target,
        };
        let root = self.inside(&target)?;
        if target.strip_prefix(&root).ok().and_then(|rel| rel.components().next()).is_some_and(|c| c.as_os_str() == ".git") {
            return Err(AppError::msg("KlikVibe doesn't write inside .git"));
        }
        Ok(target)
    }

    /// Where something new named `name` (a file or folder name, or a relative
    /// path such as "src/lib/util.ts") would go in folder `dir`: inside an open
    /// project, never in `.git`, not there yet. Every folder on the way that
    /// already exists is resolved and checked, so a symlink can't lead outside.
    pub fn fresh(&self, dir: &str, name: &str) -> AppResult<PathBuf> {
        let dir = self.existing(dir)?;
        if !dir.is_dir() {
            return Err(AppError::msg("Not a folder"));
        }
        let rel = Path::new(name.trim());
        let parts: Vec<_> = rel.components().collect();
        if parts.is_empty() || !parts.iter().all(|c| matches!(c, Component::Normal(n) if *n != ".git")) {
            return Err(AppError::msg("Use a plain name (folders with /), without .. or .git"));
        }
        let mut at = dir;
        for part in parts {
            at = at.join(part);
            if at.symlink_metadata().is_ok() {
                at = self.existing(&at.display().to_string())?;
            }
        }
        if at.symlink_metadata().is_ok() {
            return Err(AppError::msg(format!("{} already exists", name.trim())));
        }
        Ok(at)
    }

    /// The top folder of the open project holding `resolved`, if any.
    pub fn project_of(&self, resolved: &Path) -> Option<PathBuf> {
        self.inside(resolved).ok()
    }

    fn inside(&self, resolved: &Path) -> AppResult<PathBuf> {
        self.open
            .lock()
            .unwrap()
            .iter()
            .find(|root| resolved.starts_with(root))
            .cloned()
            .ok_or_else(|| AppError::msg("That file is outside the open project"))
    }
}

/// Windows' canonical paths start with `\\?\`, which other tools (and the
/// window) don't expect; drop it when the plain form means the same.
pub fn dunce(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            if !rest.starts_with("UNC\\") {
                return PathBuf::from(rest);
            }
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("klikvibe-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        dir
    }

    #[test]
    fn only_open_projects_are_reachable() {
        let dir = temp();
        let projects = Projects::new(dir.join("data"));
        let file = dir.join("src/main.rs").display().to_string();
        assert!(projects.existing(&file).is_err());
        projects.admit(&dir).unwrap();
        assert!(projects.existing(&file).is_ok());
        assert!(projects.existing(&std::env::temp_dir().display().to_string()).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_entry_is_the_link_itself_not_what_it_points_to() {
        let dir = temp();
        let projects = Projects::new(dir.join("data"));
        projects.admit(&dir).unwrap();
        let outside = std::env::temp_dir().join(format!("klikvibe-outside-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(dir.join("src"), dir.join("link")).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("away")).unwrap();
        std::os::unix::fs::symlink(dir.join("nothing"), dir.join("dangling")).unwrap();
        for name in ["link", "away", "dangling"] {
            let at = projects.entry(&dir.join(name).display().to_string()).unwrap();
            assert_eq!(at.file_name().unwrap(), name);
            assert!(at.symlink_metadata().unwrap().file_type().is_symlink());
        }
        // Through a link that leads out of the project: refused.
        assert!(projects.entry(&dir.join("away/x").display().to_string()).is_err());
        assert!(projects.entry(&dir.join("missing").display().to_string()).is_err());
        assert!(projects.entry(&outside.display().to_string()).is_err());
        std::fs::remove_dir_all(&outside).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writes_stay_out_of_git_and_parent_folders() {
        let dir = temp();
        let projects = Projects::new(dir.join("data"));
        projects.admit(&dir).unwrap();
        let ok = dir.join("src/new.rs").display().to_string();
        assert!(projects.writable(&ok).is_ok());
        assert!(projects.writable(&dir.join(".git/config").display().to_string()).is_err());
        assert!(projects.writable(&format!("{}/src/../../escape.txt", dir.display())).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn new_entries_stay_inside_the_project() {
        let dir = temp();
        let projects = Projects::new(dir.join("data"));
        projects.admit(&dir).unwrap();
        let root = dir.display().to_string();
        assert_eq!(projects.fresh(&root, "src/lib/new.ts").unwrap(), dunce(dir.canonicalize().unwrap()).join("src/lib/new.ts"));
        assert!(projects.fresh(&root, "src/main.rs").is_err(), "exists");
        assert!(projects.fresh(&root, "../escape.txt").is_err());
        assert!(projects.fresh(&root, ".git/config").is_err());
        assert!(projects.fresh(&root, "/etc/x").is_err());
        assert!(projects.fresh(&root, "  ").is_err());
        #[cfg(unix)]
        {
            let outside = std::env::temp_dir().join(format!("klikvibe-outside-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, dir.join("link")).unwrap();
            assert!(projects.fresh(&root, "link/x.txt").is_err(), "a symlink can't lead outside");
            std::fs::remove_dir_all(outside).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recent_list_gates_reopening() {
        let dir = temp();
        let projects = Projects::new(dir.join("data"));
        let root = dir.display().to_string();
        assert!(projects.reopen(&root).is_err());
        let p = projects.admit(&dir).unwrap();
        assert!(projects.reopen(&p.root).is_ok());
        // The list survives a restart.
        let again = Projects::new(dir.join("data"));
        assert_eq!(again.recent()[0].root, p.root);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
