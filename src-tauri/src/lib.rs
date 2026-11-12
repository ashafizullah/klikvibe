mod commands;
mod error;
mod files;
mod git;
mod mcp;
mod media;
mod projects;
mod remote;
mod terminal;
mod updater;
mod watch;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
/// The window has unsaved edits (the window says so with `set_unsaved`): quitting asks first.
static UNSAVED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn set_unsaved(unsaved: bool) {
    UNSAVED.store(unsaved, Ordering::Relaxed);
}

/// Quits, after the window has asked about unsaved edits.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Quit or the window's close button: with unsaved edits the window asks first ("quit-asked").
fn ask_to_quit(app: &tauri::AppHandle) -> bool {
    if UNSAVED.load(Ordering::Relaxed) {
        let _ = app.emit("quit-asked", ());
        return false;
    }
    true
}

pub fn run() {
    // What the terminals print, kept for KlikVibe Remote (see remote/hub.rs).
    let hub = Arc::new(remote::hub::Hub::default());
    let terminals = terminal::Terminals::new(hub.clone());
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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Images, video, audio and PDFs from the open projects (see media.rs).
        .register_asynchronous_uri_scheme_protocol("media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(media::serve(&app.state::<projects::Projects>(), &request)));
        })
        .manage(terminals)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && !ask_to_quit(window.app_handle())
            {
                api.prevent_close();
            }
        })
        .on_menu_event(|app, event| {
            if event.id() == "quit" && ask_to_quit(app) {
                app.exit(0);
            }
        })
        .manage(watch::Watchers::default())
        .setup(|app| {
            let projects = projects::Projects::new(app.path().app_data_dir()?);
            if let Some(dir) = folder_arg(std::env::args().skip(1), None)
                && let Ok(project) = projects.admit(&dir)
            {
                *projects.launch.lock().unwrap() = Some(project);
            }
            app.manage(projects);
            // macOS: the menu's Quit (⌘Q) ends the app at once, so it is swapped for one that asks
            // about unsaved edits first.
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, MenuItem, MenuItemKind};
                let menu = Menu::default(app.handle())?;
                if let Some(MenuItemKind::Submenu(first)) = menu.items()?.into_iter().next() {
                    for item in first.items()? {
                        if let MenuItemKind::Predefined(p) = &item
                            && let Ok(text) = p.text()
                            && text.starts_with("Quit")
                        {
                            first.remove(&item)?;
                            first.append(&MenuItem::with_id(app, "quit", text, true, Some("CmdOrCtrl+Q"))?)?;
                        }
                    }
                }
                app.set_menu(menu)?;
            }
            app.manage(mcp::Mcp::new(app.path().app_data_dir()?));
            app.manage(remote::Remote::new(app.path().app_data_dir()?, hub));
            remote::Remote::init(app.handle());
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
            commands::path_kinds,
            commands::find_definitions,
            commands::outside_kinds,
            commands::open_outside,
            commands::list_files,
            commands::search_text,
            commands::read_file,
            commands::create_entry,
            commands::rename_entry,
            commands::delete_entry,
            commands::write_file,
            commands::open_externally,
            commands::clipboard_text,
            commands::mcp_start,
            commands::mcp_stop,
            commands::mcp_link,
            commands::mcp_reply,
            commands::git_status,
            commands::git_log,
            commands::git_diff,
            commands::git_show,
            commands::git_commit_files,
            commands::git_commit_file_diff,
            commands::git_commit,
            commands::git_stage,
            commands::git_unstage,
            commands::git_push,
            commands::git_pull,
            commands::git_init,
            commands::git_remote,
            commands::worktree_list,
            commands::worktree_add,
            commands::worktree_open,
            commands::worktree_merge,
            commands::worktree_remove,
            commands::checkpoint_create,
            commands::checkpoint_list,
            commands::checkpoint_restore,
            commands::checkpoint_clear,
            commands::checkpoint_prune,
            commands::checkpoint_size,
            commands::checkpoint_skipped,
            commands::git_discard,
            commands::git_staged_hunks,
            commands::git_hunk,
            commands::git_hunk_restore,
            commands::git_ignore,
            commands::terminal_open,
            commands::terminal_write,
            commands::terminal_resize,
            commands::terminal_close,
            commands::terminal_foreground,
            commands::remote_status,
            commands::remote_set_enabled,
            commands::remote_pair_start,
            commands::remote_pair_cancel,
            commands::remote_pair_confirm,
            commands::remote_forget_phone,
            commands::remote_unlink,
            commands::remote_projects,
            commands::remote_attention,
            commands::remote_agents,
            commands::remote_term_created,
            commands::remote_term_snapshot,
            commands::check_updates,
            commands::install_update,
            commands::prepare_update,
            commands::set_update_mode,
            commands::pending_update,
            commands::update_changelog,
            set_unsaved,
            quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building KlikVibe")
        .run(|_, event| {
            // An update downloaded in the background is installed as the app quits.
            if let tauri::RunEvent::Exit = event {
                updater::install_on_exit();
            }
        });
}
