//! Interactive terminal sessions (a PTY running the user's login shell) for
//! the in-app terminal panel. The app only relays keystrokes and output; it
//! never drives what runs inside.
use crate::error::{AppError, AppResult};
use base64::Engine;
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

use crate::remote::hub::Hub;

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TerminalEvent {
    /// Raw output bytes, base64-encoded.
    Data {
        data: String,
    },
    Exit {
        code: Option<u32>,
    },
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    /// Input on its way to the program, written by a thread of the session's own: a write
    /// that blocks (a program not reading its input) holds up nothing else, and the order stays.
    input: std::sync::mpsc::Sender<Vec<u8>>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

/// The sessions, and the Remote hub that the phone sees them through (it keeps
/// each one's recent output; nothing leaves the computer unless Remote is on).
pub struct Terminals {
    sessions: Mutex<HashMap<String, Session>>,
    hub: Arc<Hub>,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { rows: rows.max(2), cols: cols.max(10), pixel_width: 0, pixel_height: 0 }
}

fn shell() -> CommandBuilder {
    if cfg!(windows) {
        return CommandBuilder::new(std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into()));
    }
    let default = if cfg!(target_os = "macos") { "/bin/zsh" } else { "/bin/bash" };
    let mut cmd = CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| default.into()));
    // Login shell: GUI apps on macOS don't inherit the user's PATH.
    cmd.arg("-l");
    cmd
}

impl Terminals {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self { sessions: Mutex::default(), hub }
    }

    pub fn open(&self, cwd: &str, cols: u16, rows: u16, on_event: Channel<TerminalEvent>) -> AppResult<String> {
        let pair = native_pty_system().openpty(size(cols, rows)).map_err(|e| AppError::msg(e.to_string()))?;
        let mut cmd = shell();
        if std::path::Path::new(cwd).is_dir() {
            cmd.cwd(cwd);
        } else if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
            cmd.cwd(home);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if std::env::var_os("LANG").is_none() {
            cmd.env("LANG", "en_US.UTF-8");
        }
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| AppError::msg(e.to_string()))?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| AppError::msg(e.to_string()))?;
        let mut writer = pair.master.take_writer().map_err(|e| AppError::msg(e.to_string()))?;
        let (input, typed) = std::sync::mpsc::channel::<Vec<u8>>();
        // Ends when the session is closed (the sender goes) or the terminal is gone (the write fails).
        std::thread::spawn(move || {
            for data in typed {
                if writer.write_all(&data).is_err() || writer.flush().is_err() {
                    break;
                }
            }
        });
        let killer = child.clone_killer();
        let id = uuid::Uuid::new_v4().to_string();
        self.hub.started(&id, cols, rows);
        let hub = self.hub.clone();
        let sid = id.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        hub.output(&sid, &buf[..n]);
                        let data = base64::engine::general_purpose::STANDARD.encode(&buf[..n]);
                        if on_event.send(TerminalEvent::Data { data }).is_err() {
                            break;
                        }
                    }
                }
            }
            let code = child.wait().ok().map(|s| s.exit_code());
            hub.exited(&sid, code);
            let _ = on_event.send(TerminalEvent::Exit { code });
        });

        self.sessions.lock().unwrap().insert(id.clone(), Session { master: pair.master, input, killer });
        Ok(id)
    }

    pub fn write(&self, id: &str, data: &str) -> AppResult<()> {
        let ended = || AppError::msg("Terminal session has ended");
        // Queued, never waited for: this is called from the window's and the phone's own loops.
        self.sessions.lock().unwrap().get(id).ok_or_else(ended)?.input.send(data.as_bytes().to_vec()).map_err(|_| ended())?;
        self.hub.input(id);
        Ok(())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> AppResult<()> {
        if let Some(s) = self.sessions.lock().unwrap().get(id) {
            s.master.resize(size(cols, rows)).map_err(|e| AppError::msg(e.to_string()))?;
            self.hub.resized(id, cols.max(10), rows.max(2));
        }
        Ok(())
    }

    /// The command line of the program in front in a session (an agent, or
    /// just the shell), so the window can tell an agent waiting for the user
    /// from an idle prompt. Unix only.
    pub fn foreground(&self, id: &str) -> Option<String> {
        #[cfg(unix)]
        {
            let pid = self.sessions.lock().unwrap().get(id)?.master.process_group_leader()?;
            let out = std::process::Command::new("ps").args(["-o", "args=", "-p", &pid.to_string()]).output().ok()?;
            let args = String::from_utf8_lossy(&out.stdout).trim().to_string();
            (!args.is_empty()).then_some(args)
        }
        #[cfg(not(unix))]
        {
            let _ = id;
            None
        }
    }

    pub fn close(&self, id: &str) {
        if let Some(mut s) = self.sessions.lock().unwrap().remove(id) {
            let _ = s.killer.kill();
        }
        self.hub.closed(id);
    }
}

impl Drop for Terminals {
    fn drop(&mut self) {
        for (_, mut s) in self.sessions.lock().unwrap().drain() {
            let _ = s.killer.kill();
        }
    }
}
