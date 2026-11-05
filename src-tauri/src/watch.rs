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

/// Splits raw event paths into what the window cares about.
pub fn sort(root: &Path, paths: impl IntoIterator<Item = PathBuf>) -> Changed {
    let git_dir = root.join(".git");
    let mut out = Changed { root: root.display().to_string(), paths: Vec::new(), many: false, git: false };
    for p in paths {
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
        let mut debouncer = new_debouncer(Duration::from_millis(250), move |res: DebounceEventResult| {
            if let Ok(events) = res {
                let changed = sort(&watched, events.into_iter().map(|e| e.path));
                if changed.git || !changed.paths.is_empty() {
                    let _ = app.emit("fs-changed", changed);
                }
            }
        })
        .map_err(|e| AppError::msg(e.to_string()))?;
        debouncer.watcher().watch(&root, RecursiveMode::Recursive).map_err(|e| AppError::msg(e.to_string()))?;
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
        let c = sort(&root, [root.join(".git/objects/ab/cdef")]);
        assert!(!c.git && c.paths.is_empty());
        let c = sort(&root, [root.join(".git/HEAD"), root.join("src/a.ts"), root.join("src/a.ts")]);
        assert!(c.git);
        assert_eq!(c.paths, [root.join("src/a.ts").display().to_string()]);
    }

    #[test]
    fn floods_become_many() {
        let root = PathBuf::from("/p");
        let c = sort(&root, (0..400).map(|i| root.join(format!("node_modules/{i}"))));
        assert!(c.many && c.paths.is_empty());
    }
}
