//! Self-update from GitHub Releases. The release workflow publishes a signed
//! `latest.json`; the updater only installs packages signed with our key.
//! A check runs shortly after launch and once a day; the window offers what
//! it finds and installs only when the user agrees.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

const STARTUP_DELAY: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

static CHECKING: AtomicBool = AtomicBool::new(false);
/// The newer version found by the last check, until it is installed.
static AVAILABLE: Mutex<Option<Update>> = Mutex::new(None);

/// A new version: its number, the running one, and the release notes.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    version: String,
    current: String,
    notes: String,
}

impl Release {
    fn of(update: &Update) -> Self {
        Release {
            version: update.version.clone(),
            current: update.current_version.clone(),
            notes: notes(update.body.as_deref().unwrap_or("")),
        }
    }
}

/// The outcome of a check.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Check {
    Available(Release),
    UpToDate {
        current: String,
    },
    /// A check is already running.
    Busy,
}

/// Checks shortly after launch and then once a day, and tells the window about a new version.
pub fn start_background_checks(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(STARTUP_DELAY);
        loop {
            if let Ok(Check::Available(release)) = tauri::async_runtime::block_on(check(&app)) {
                let _ = app.emit("update-available", release);
            }
            std::thread::sleep(INTERVAL);
        }
    });
}

/// Looks for a newer version and keeps it for `install`.
pub async fn check(app: &AppHandle) -> Result<Check, String> {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return Ok(Check::Busy);
    }
    let result = async { app.updater()?.check().await }.await;
    CHECKING.store(false, Ordering::SeqCst);
    match result.map_err(|e| e.to_string())? {
        Some(update) => {
            let release = Release::of(&update);
            *AVAILABLE.lock().unwrap() = Some(update);
            Ok(Check::Available(release))
        }
        None => Ok(Check::UpToDate { current: app.package_info().version.to_string() }),
    }
}

/// Downloads and installs what `check` found, then restarts into it.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = AVAILABLE.lock().unwrap().take().ok_or("No update to install; check again")?;
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart()
}

/// The changelog part of the release notes (Markdown): the install
/// instructions after "## Download" are for people on the release page.
fn notes(body: &str) -> String {
    let end = body.lines().position(|l| l.trim_start().starts_with("## Download")).unwrap_or(usize::MAX);
    body.lines().take(end).collect::<Vec<_>>().join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::notes;

    #[test]
    fn keeps_the_changelog_part_of_the_notes() {
        let body = "\n- Multiple terminals\n- Faster tree\n\n## Download\n\n**macOS**: download `KlikVibe.dmg`";
        assert_eq!(notes(body), "- Multiple terminals\n- Faster tree");
        assert_eq!(notes("## Download\n\n**Windows**: download it"), "");
    }
}
