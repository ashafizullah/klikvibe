mod commands;
mod error;
mod files;
mod git;
mod projects;
mod terminal;
mod updater;
mod watch;

use std::path::PathBuf;
use tauri::{Emitter, Manager};

/// The first folder among launch arguments (`klikvibe .`), resolved against `cwd`.
fn folder_arg(args: impl IntoIterator<Item = String>, cwd: Option<&str>) -> Option<PathBuf> {
    args.into_iter().filter(|a| !a.starts_with('-')).map(PathBuf::from).find_map(|p| {
        let p = match cwd {
            Some(dir) if p.is_relative() => PathBuf::from(dir).join(p),
            _ => p,
        };
        p.is_dir().then_some(p)
    })
}

fn focus(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            focus(app);
            // `klikvibe <folder>` while running: open it in the window.
            if let Some(dir) = folder_arg(args.into_iter().skip(1), Some(&cwd))
                && let Ok(project) = app.state::<projects::Projects>().admit(&dir)
            {
                let _ = app.emit("open-project", project);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(terminal::Terminals::default())
        .manage(watch::Watchers::default())
        .setup(|app| {
            let projects = projects::Projects::new(app.path().app_data_dir()?);
            if let Some(dir) = folder_arg(std::env::args().skip(1), None)
                && let Ok(project) = projects.admit(&dir)
            {
                *projects.launch.lock().unwrap() = Some(project);
            }
            app.manage(projects);
            // Debug builds are not released, so they have nothing to update to.
            if !cfg!(debug_assertions) {
                updater::start_background_checks(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_project,
            commands::open_recent,
            commands::recent_projects,
            commands::forget_recent,
            commands::take_launch_project,
            commands::watch_project,
            commands::unwatch_project,
            commands::list_dir,
            commands::read_file,
            commands::write_file,
            commands::git_status,
            commands::git_log,
            commands::git_diff,
            commands::git_show,
            commands::git_commit,
            commands::git_push,
            commands::git_pull,
            commands::git_init,
            commands::terminal_open,
            commands::terminal_write,
            commands::terminal_resize,
            commands::terminal_close,
            commands::check_updates,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KlikVibe");
}
