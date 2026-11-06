//! What the window can ask for. Every path is checked against the open
//! projects (see projects.rs); slow work runs off the main thread.
use crate::error::{AppError, AppResult};
use crate::files::{self, Entry, FileList, FileText, TextSearch};
use crate::git::{self, Checkpoint, Commit, CommitFile, Status, Worktree};
use crate::mcp::{Mcp, McpCall, McpInfo, McpLink};
use crate::projects::{Project, Projects, RecentProject};
use crate::terminal::{TerminalEvent, Terminals};
use crate::updater;
use crate::watch::Watchers;
use std::path::{Component, Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| AppError::msg(e.to_string()))?
}

/// A path relative to the project, as Git reports it: no `..`, not absolute.
fn relative(path: &str) -> AppResult<&str> {
    let p = Path::new(path);
    if path.is_empty() || p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(AppError::msg("Invalid path"));
    }
    Ok(path)
}

// ---- projects ----

#[tauri::command]
pub async fn pick_project(app: AppHandle, projects: State<'_, Projects>) -> AppResult<Option<Project>> {
    let picked = tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    let Some(dir) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    projects.admit(&dir).map(Some)
}

#[tauri::command]
pub fn open_recent(projects: State<'_, Projects>, root: String) -> AppResult<Project> {
    projects.reopen(&root)
}

#[tauri::command]
pub fn recent_projects(projects: State<'_, Projects>) -> Vec<RecentProject> {
    projects.recent()
}

#[tauri::command]
pub fn forget_recent(projects: State<'_, Projects>, root: String) {
    projects.forget(&root)
}

#[tauri::command]
pub fn take_launch_project(projects: State<'_, Projects>) -> Option<Project> {
    projects.launch.lock().unwrap().take()
}

#[tauri::command]
pub fn watch_project(app: AppHandle, projects: State<'_, Projects>, watchers: State<'_, Watchers>, root: String) -> AppResult<()> {
    watchers.watch(app, projects.root(&root)?)
}

#[tauri::command]
pub fn unwatch_project(projects: State<'_, Projects>, watchers: State<'_, Watchers>, root: String) -> AppResult<()> {
    watchers.unwatch(&projects.root(&root)?);
    Ok(())
}

// ---- files ----

#[tauri::command]
pub async fn list_dir(projects: State<'_, Projects>, path: String) -> AppResult<Vec<Entry>> {
    let dir = projects.existing(&path)?;
    blocking(move || files::list(&dir)).await
}

/// For each path: "file", "dir", or null when it isn't there (or is outside
/// the open projects). For links in terminal output.
#[tauri::command]
pub fn path_kinds(projects: State<'_, Projects>, paths: Vec<String>) -> Vec<Option<&'static str>> {
    paths.iter().map(|p| projects.existing(p).ok().map(|r| if r.is_dir() { "dir" } else { "file" })).collect()
}

/// Every file in an open project (`.gitignore` applied), for search by name.
#[tauri::command]
pub async fn list_files(projects: State<'_, Projects>, root: String) -> AppResult<FileList> {
    let root = projects.root(&root)?;
    blocking(move || files::list_all(&root)).await
}

/// Lines containing `query` in an open project's text files, optionally only in files matching `include` globs.
#[tauri::command]
pub async fn search_text(projects: State<'_, Projects>, root: String, query: String, include: Option<String>) -> AppResult<TextSearch> {
    let root = projects.root(&root)?;
    blocking(move || files::search_text(&root, &query, include.as_deref())).await
}

/// A new empty file, or a folder, named `name` (folders on the way allowed) in folder `dir`.
#[tauri::command]
pub async fn create_entry(projects: State<'_, Projects>, dir: String, name: String, folder: bool) -> AppResult<String> {
    let path = projects.fresh(&dir, &name)?;
    let shown = path.display().to_string();
    blocking(move || files::create(&path, folder)).await?;
    Ok(shown)
}

/// Renames a file or folder in place; returns the new path.
#[tauri::command]
pub async fn rename_entry(projects: State<'_, Projects>, path: String, name: String) -> AppResult<String> {
    let from = projects.existing(&path)?;
    let root = projects.project_of(&from).ok_or_else(|| AppError::msg("That is outside the open project"))?;
    if from == root {
        return Err(AppError::msg("The project folder itself can't be renamed here"));
    }
    let name = files::plain_name(&name)?.to_string();
    let parent = from.parent().ok_or_else(|| AppError::msg("Invalid path"))?.display().to_string();
    let to = projects.fresh(&parent, &name)?;
    let shown = to.display().to_string();
    blocking(move || Ok(std::fs::rename(&from, &to)?)).await?;
    Ok(shown)
}

/// Deletes a file or a folder with everything in it.
#[tauri::command]
pub async fn delete_entry(projects: State<'_, Projects>, path: String) -> AppResult<()> {
    let target = projects.existing(&path)?;
    let root = projects.project_of(&target).ok_or_else(|| AppError::msg("That is outside the open project"))?;
    if target == root || target.strip_prefix(&root).is_ok_and(|rel| rel.components().next().is_some_and(|c| c.as_os_str() == ".git")) {
        return Err(AppError::msg("KlikVibe doesn't delete the project folder or anything in .git"));
    }
    blocking(move || {
        if target.is_dir() {
            std::fs::remove_dir_all(&target)?
        } else {
            std::fs::remove_file(&target)?
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn read_file(projects: State<'_, Projects>, path: String) -> AppResult<FileText> {
    let file = projects.existing(&path)?;
    blocking(move || files::read(&file)).await
}

#[tauri::command]
pub async fn write_file(projects: State<'_, Projects>, path: String, text: String) -> AppResult<()> {
    let file = projects.writable(&path)?;
    blocking(move || files::write(&file, &text)).await
}

/// Opens a media file (image, video, audio, PDF) in its default app. Nothing
/// else: opening an arbitrary project file this way could run it.
#[tauri::command]
pub fn open_externally(app: AppHandle, projects: State<'_, Projects>, path: String) -> AppResult<()> {
    let file = projects.existing(&path)?;
    if crate::media::mime(&file).is_none() {
        return Err(AppError::msg("Only images, video, audio and PDFs open this way"));
    }
    app.opener().open_path(file.display().to_string(), None::<&str>).map_err(|e| AppError::msg(e.to_string()))
}

/// "~/x" with the home folder; other paths as they are.
fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// For absolute paths printed in a terminal that lie outside the open projects:
/// "media" (an image, video, audio or PDF), "file", "dir", or null when not there.
#[tauri::command]
pub fn outside_kinds(paths: Vec<String>) -> Vec<Option<&'static str>> {
    paths
        .iter()
        .map(|p| {
            let path = expand_home(p);
            if !path.is_absolute() {
                return None;
            }
            let meta = std::fs::metadata(&path).ok()?;
            Some(if meta.is_dir() {
                "dir"
            } else if crate::media::mime(&path).is_some() {
                "media"
            } else {
                "file"
            })
        })
        .collect()
}

/// Opens a path from outside the open projects: an image, video, audio file or
/// PDF in its default app; anything else (which could run something if opened)
/// is shown in Finder/Explorer.
#[tauri::command]
pub fn open_outside(app: AppHandle, path: String) -> AppResult<()> {
    let file = expand_home(&path).canonicalize().map_err(|_| AppError::msg(format!("Not found: {path}")))?;
    let shown = file.display().to_string();
    let done = if file.is_file() && crate::media::mime(&file).is_some() {
        app.opener().open_path(shown, None::<&str>)
    } else {
        app.opener().reveal_item_in_dir(&file)
    };
    done.map_err(|e| AppError::msg(e.to_string()))
}

// ---- git ----

fn root_of(projects: &Projects, root: &str) -> AppResult<PathBuf> {
    projects.root(root)
}

#[tauri::command]
pub async fn git_status(projects: State<'_, Projects>, root: String) -> AppResult<Status> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::status(&root)).await
}

#[tauri::command]
pub async fn git_log(projects: State<'_, Projects>, root: String, limit: u32, all: bool) -> AppResult<Vec<Commit>> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::log(&root, limit, all)).await
}

#[tauri::command]
pub async fn git_diff(projects: State<'_, Projects>, root: String, path: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    let path = relative(&path)?.to_string();
    blocking(move || git::diff(&root, &path)).await
}

#[tauri::command]
pub async fn git_show(projects: State<'_, Projects>, root: String, hash: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::show(&root, &hash)).await
}

#[tauri::command]
pub async fn git_commit_files(projects: State<'_, Projects>, root: String, hash: String) -> AppResult<Vec<CommitFile>> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::commit_files(&root, &hash)).await
}

#[tauri::command]
pub async fn git_commit_file_diff(
    projects: State<'_, Projects>,
    root: String,
    hash: String,
    path: String,
    from: Option<String>,
) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    let path = relative(&path)?.to_string();
    let from = from.as_deref().map(relative).transpose()?.map(String::from);
    blocking(move || git::commit_file_diff(&root, &hash, &path, from.as_deref())).await
}

/// Commits what is staged, or (`all`) everything.
#[tauri::command]
pub async fn git_commit(projects: State<'_, Projects>, root: String, message: String, all: bool) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::commit(&root, &message, all)).await
}

/// Paths as Git reports them (relative to the project), checked before they reach Git.
fn rel_paths(paths: &[String]) -> AppResult<Vec<String>> {
    if paths.is_empty() {
        return Err(AppError::msg("No files given"));
    }
    paths.iter().map(|p| relative(p).map(String::from)).collect()
}

#[tauri::command]
pub async fn git_stage(projects: State<'_, Projects>, root: String, paths: Vec<String>) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    let paths = rel_paths(&paths)?;
    blocking(move || git::stage(&root, &paths)).await
}

#[tauri::command]
pub async fn git_unstage(projects: State<'_, Projects>, root: String, paths: Vec<String>) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    let paths = rel_paths(&paths)?;
    blocking(move || git::unstage(&root, &paths)).await
}

#[tauri::command]
pub async fn git_push(projects: State<'_, Projects>, root: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::push(&root)).await
}

#[tauri::command]
pub async fn git_pull(projects: State<'_, Projects>, root: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::pull(&root)).await
}

#[tauri::command]
pub async fn git_discard(
    projects: State<'_, Projects>,
    root: String,
    path: String,
    from: Option<String>,
    untracked: bool,
) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    let path = relative(&path)?.to_string();
    let from = from.as_deref().map(relative).transpose()?.map(String::from);
    blocking(move || git::discard(&root, &path, from.as_deref(), untracked)).await
}

#[tauri::command]
pub async fn git_ignore(projects: State<'_, Projects>, root: String, path: String, dir: bool) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    let path = relative(&path)?.to_string();
    // Through `writable`, so a .gitignore symlinked out of the project is refused.
    let file = projects.writable(&root.join(".gitignore").display().to_string())?;
    blocking(move || git::ignore(&file, &path, dir)).await
}

#[derive(serde::Serialize)]
pub struct CheckpointMade {
    checkpoint: Option<Checkpoint>,
    /// The oldest went to stay within the limit: the list has new hashes.
    pruned: bool,
}

/// Takes a checkpoint of the working tree (see git.rs); `checkpoint` is None when
/// nothing changed. Keeps at most about `keep` of them (0: all).
#[tauri::command]
pub async fn checkpoint_create(
    projects: State<'_, Projects>,
    root: String,
    reason: String,
    note: Option<String>,
    keep: u32,
) -> AppResult<CheckpointMade> {
    let root = projects.root(&root)?;
    blocking(move || {
        let (checkpoint, pruned) = git::checkpoint_keeping(&root, &reason, note.as_deref().unwrap_or(""), keep)?;
        Ok(CheckpointMade { checkpoint, pruned })
    })
    .await
}

#[tauri::command]
pub async fn checkpoint_list(projects: State<'_, Projects>, root: String, limit: u32) -> AppResult<Vec<Checkpoint>> {
    let root = projects.root(&root)?;
    blocking(move || git::checkpoints(&root, limit)).await
}

/// Keeps only the newest `keep` checkpoints; true when some went.
#[tauri::command]
pub async fn checkpoint_prune(projects: State<'_, Projects>, root: String, keep: u32) -> AppResult<bool> {
    let root = projects.root(&root)?;
    blocking(move || git::prune_checkpoints(&root, keep)).await
}

/// What the checkpoints take on disk, in bytes (null when Git can't say).
#[tauri::command]
pub async fn checkpoint_size(projects: State<'_, Projects>, root: String) -> AppResult<Option<u64>> {
    let root = projects.root(&root)?;
    blocking(move || git::checkpoint_size(&root)).await
}

/// Puts the files back as they were at a checkpoint (after saving the current state as one).
#[tauri::command]
pub async fn checkpoint_restore(projects: State<'_, Projects>, root: String, hash: String) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::restore_checkpoint(&root, &hash)).await
}

#[tauri::command]
pub async fn checkpoint_clear(projects: State<'_, Projects>, root: String) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::clear_checkpoints(&root)).await
}

/// The project's remote URL (origin, else the first), for links to GitHub, GitLab or Bitbucket.
#[tauri::command]
pub async fn git_remote(projects: State<'_, Projects>, root: String) -> AppResult<Option<String>> {
    let root = root_of(&projects, &root)?;
    blocking(move || Ok(git::remote_url(&root))).await
}

// ---- worktrees ----

#[tauri::command]
pub async fn worktree_list(projects: State<'_, Projects>, root: String) -> AppResult<Vec<Worktree>> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::worktrees(&root)).await
}

/// Makes a worktree for a new task and opens it as a project (the user asked for it).
#[tauri::command]
pub async fn worktree_add(projects: State<'_, Projects>, root: String, name: String) -> AppResult<Project> {
    let root = root_of(&projects, &root)?;
    let dir = blocking(move || git::add_worktree(&root, &name)).await?;
    projects.admit(&dir)
}

/// Opens one of the repository's worktrees as a project; only folders Git lists as its worktrees.
#[tauri::command]
pub async fn worktree_open(projects: State<'_, Projects>, root: String, path: String) -> AppResult<Project> {
    let root = root_of(&projects, &root)?;
    let wanted = path.clone();
    let known = blocking(move || git::worktrees(&root)).await?;
    if !known.iter().any(|w| w.path == wanted) {
        return Err(AppError::msg("That folder isn't one of this repository's worktrees"));
    }
    projects.admit(Path::new(&path))
}

#[tauri::command]
pub async fn worktree_merge(projects: State<'_, Projects>, root: String, branch: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::merge_task(&root, &branch)).await
}

#[tauri::command]
pub async fn worktree_remove(projects: State<'_, Projects>, root: String, path: String, force: bool) -> AppResult<()> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::remove_worktree(&root, &path, force)).await
}

#[tauri::command]
pub async fn git_init(projects: State<'_, Projects>, root: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::init(&root)).await
}

// ---- terminal ----

#[tauri::command]
pub fn terminal_open(
    projects: State<'_, Projects>,
    terminals: State<'_, Terminals>,
    cwd: String,
    cols: u16,
    rows: u16,
    on_event: Channel<TerminalEvent>,
) -> AppResult<String> {
    // Shells start in an open project, nowhere else.
    let root = projects.root(&cwd)?;
    terminals.open(&root.display().to_string(), cols, rows, on_event)
}

#[tauri::command]
pub fn terminal_write(terminals: State<'_, Terminals>, id: String, data: String) -> AppResult<()> {
    terminals.write(&id, &data)
}

#[tauri::command]
pub fn terminal_resize(terminals: State<'_, Terminals>, id: String, cols: u16, rows: u16) -> AppResult<()> {
    terminals.resize(&id, cols, rows)
}

#[tauri::command]
pub fn terminal_foreground(terminals: State<'_, Terminals>, id: String) -> Option<String> {
    terminals.foreground(&id)
}

#[tauri::command]
pub fn terminal_close(terminals: State<'_, Terminals>, id: String) {
    terminals.close(&id)
}

// ---- clipboard ----

/// The clipboard's text, for Paste in the editor's right-click menu (the web
/// view can't read it without its own prompt). Empty when it holds no text.
#[tauri::command]
pub async fn clipboard_text() -> AppResult<String> {
    blocking(|| match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(text) => Ok(text),
        Err(arboard::Error::ContentNotAvailable) => Ok(String::new()),
        Err(e) => Err(AppError::msg(e.to_string())),
    })
    .await
}

// ---- MCP ----

#[tauri::command]
pub async fn mcp_start(
    mcp: State<'_, Mcp>,
    tools: serde_json::Value,
    instructions: String,
    on_call: Channel<McpCall>,
) -> AppResult<McpInfo> {
    mcp.start(tools, instructions, on_call).await
}

#[tauri::command]
pub fn mcp_stop(mcp: State<'_, Mcp>) {
    mcp.stop()
}

#[tauri::command]
pub fn mcp_link(mcp: State<'_, Mcp>) -> McpLink {
    mcp.link()
}

#[tauri::command]
pub fn mcp_reply(mcp: State<'_, Mcp>, id: u64, result: serde_json::Value) {
    mcp.reply(id, result)
}

// ---- updates ----

#[tauri::command]
pub async fn check_updates(app: AppHandle) -> AppResult<updater::Check> {
    updater::check(&app).await.map_err(AppError::msg)
}

/// Installs the version the last check found (or the one downloaded in the background) and restarts.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    updater::install_available(&app).await.map_err(AppError::msg)
}

/// "auto", "notify" or "off".
#[tauri::command]
pub fn set_update_mode(mode: String) {
    updater::set_mode(&mode)
}

/// A version downloaded in the background, waiting to be installed.
#[tauri::command]
pub fn pending_update() -> Option<updater::Release> {
    updater::pending()
}

/// The changelog of `version` in the interface language, from the repository at its tag.
#[tauri::command]
pub async fn update_changelog(version: String, locale: String) -> AppResult<String> {
    updater::changelog(&version, &locale).await.map_err(AppError::msg)
}

#[cfg(test)]
mod tests {
    use super::relative;

    #[test]
    fn git_paths_stay_relative() {
        assert!(relative("src/app.ts").is_ok());
        assert!(relative("../secret").is_err());
        assert!(relative("/etc/passwd").is_err());
        assert!(relative("").is_err());
    }
}
