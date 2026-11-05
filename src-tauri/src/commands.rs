//! What the window can ask for. Every path is checked against the open
//! projects (see projects.rs); slow work runs off the main thread.
use crate::error::{AppError, AppResult};
use crate::files::{self, Entry, FileText};
use crate::git::{self, Commit, CommitFile, Status};
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

#[tauri::command]
pub async fn git_commit(projects: State<'_, Projects>, root: String, message: String) -> AppResult<String> {
    let root = root_of(&projects, &root)?;
    blocking(move || git::commit(&root, &message)).await
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
