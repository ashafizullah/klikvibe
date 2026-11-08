//! Tells the window when files change on disk, so an agent's edits show up
//! in the tree, the open files and the Git panel without reloading.
use crate::error::{AppError, AppResult};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer, notify::RecommendedWatcher, notify::RecursiveMode};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// More changed paths than this in one batch are sent as "many".
const MAX_PATHS: usize = 300;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Changed {
    pub root: String,
    /// Changed files and folders outside `.git`; empty when `many`.
    pub paths: Vec<String>,
    pub many: bool,
    /// Something Git tracks moved: a commit, a checkout, the index.
    pub git: bool,
}

#[derive(Default)]
pub struct Watchers(Mutex<HashMap<PathBuf, Debouncer<RecommendedWatcher>>>);

/// A linked worktree's own Git folder: there `.git` is a file ("gitdir: <main>/.git/worktrees/<id>"),
/// and a commit or `git add` in it changes only that folder and the main repository, both outside
/// the project folder. None for an ordinary repository.
pub fn linked_git_dir(root: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(root.join(".git")).ok()?;
    let dir = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    Some(if dir.is_absolute() { dir } else { root.join(dir) })
}

/// Splits raw event paths into what the window cares about. `linked`: a worktree's own Git folder
/// (see `linked_git_dir`), where HEAD, the index and the HEAD log mean a commit, add or checkout.
pub fn sort(root: &Path, linked: Option<&Path>, paths: impl IntoIterator<Item = PathBuf>) -> Changed {
    let git_dir = root.join(".git");
    let mut out = Changed { root: root.display().to_string(), paths: Vec::new(), many: false, git: false };
    for p in paths {
        if let Some(rel) = linked.and_then(|l| p.strip_prefix(l).ok()) {
            let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default();
            if matches!(first.as_str(), "HEAD" | "index" | "logs" | "ORIG_HEAD" | "MERGE_HEAD") {
                out.git = true;
            }
            continue;
        }
        if let Ok(rel) = p.strip_prefix(&git_dir) {
            // Objects and logs churn on every command; HEAD, the index and refs mean a real change.
            let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default();
            if matches!(first.as_str(), "HEAD" | "index" | "refs" | "packed-refs" | "FETCH_HEAD" | "MERGE_HEAD") {
                out.git = true;
            }
            continue;
        }
        out.git = true;
        if out.paths.len() >= MAX_PATHS {
            out.many = true;
            continue;
        }
        let s = p.display().to_string();
        if !out.paths.contains(&s) {
            out.paths.push(s);
        }
    }
    if out.many {
        out.paths.clear();
    }
    out
}

impl Watchers {
    pub fn watch(&self, app: AppHandle, root: PathBuf) -> AppResult<()> {
        let mut map = self.0.lock().unwrap();
        if map.contains_key(&root) {
            return Ok(());
        }
        let watched = root.clone();
        let linked = linked_git_dir(&root);
        let linked_dir = linked.clone();
        let mut debouncer = new_debouncer(Duration::from_millis(250), move |res: DebounceEventResult| {
            if let Ok(events) = res {
                let changed = sort(&watched, linked_dir.as_deref(), events.into_iter().map(|e| e.path));
                if changed.git || !changed.paths.is_empty() {
                    let _ = app.emit("fs-changed", changed);
                }
            }
        })
        .map_err(|e| AppError::msg(e.to_string()))?;
        debouncer.watcher().watch(&root, RecursiveMode::Recursive).map_err(|e| AppError::msg(e.to_string()))?;
        // A task's worktree: its commits happen in its Git folder in the main repository.
        if let Some(dir) = linked.filter(|d| d.is_dir()) {
            let _ = debouncer.watcher().watch(&dir, RecursiveMode::Recursive);
        }
        map.insert(root, debouncer);
        Ok(())
    }

    pub fn unwatch(&self, root: &Path) {
        self.0.lock().unwrap().remove(root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_internals_only_count_when_they_matter() {
        let root = PathBuf::from("/p");
        let c = sort(&root, None, [root.join(".git/objects/ab/cdef")]);
        assert!(!c.git && c.paths.is_empty());
        let c = sort(&root, None, [root.join(".git/HEAD"), root.join("src/a.ts"), root.join("src/a.ts")]);
        assert!(c.git);
        assert_eq!(c.paths, [root.join("src/a.ts").display().to_string()]);
    }

    #[test]
    fn a_worktree_commit_counts_as_git() {
        let root = PathBuf::from("/repo-task");
        let linked = PathBuf::from("/repo/.git/worktrees/task");
        let c = sort(&root, Some(&linked), [linked.join("logs/HEAD")]);
        assert!(c.git && c.paths.is_empty());
        let c = sort(&root, Some(&linked), [linked.join("COMMIT_EDITMSG")]);
        assert!(!c.git && c.paths.is_empty());
        let dir = std::env::temp_dir().join(format!("klikvibe-wt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".git"), "gitdir: /repo/.git/worktrees/task\n").unwrap();
        assert_eq!(linked_git_dir(&dir), Some(linked));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn floods_become_many() {
        let root = PathBuf::from("/p");
        let c = sort(&root, None, (0..400).map(|i| root.join(format!("node_modules/{i}"))));
        assert!(c.many && c.paths.is_empty());
    }
}
