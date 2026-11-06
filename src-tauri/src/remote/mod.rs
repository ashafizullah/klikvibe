//! KlikVibe Remote: the phone app sees the open projects and their terminals,
//! and types into them, from the same Wi-Fi (or, with Pro, from anywhere
//! through the relay). See PROTOCOL.md. Off until the user turns it on.

pub mod cloud;
pub mod core;
pub mod hub;
pub mod noise;
pub mod relay;
pub mod server;
pub mod session;
pub mod store;

use crate::git;
use crate::projects::Projects;
use crate::terminal::Terminals;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};

use self::core::Core;
use self::hub::Hub;
use self::store::{Config, Phone};

/// How often the account is checked: a phone removed in the app loses LAN access within this.
const HEARTBEAT: Duration = Duration::from_secs(60);
const PAIR_POLL: Duration = Duration::from_secs(2);

/// The window listens for this to refresh the Remote dialog.
pub const CHANGED: &str = "remote-changed";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PhoneView {
    pub id: String,
    pub name: String,
    pub platform: Option<String>,
    pub connected: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairView {
    pub url: String,
    /// The QR code of `url`, as SVG.
    pub svg: String,
    pub expires_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub port: Option<u16>,
    pub addresses: Vec<String>,
    pub linked: bool,
    pub account: Option<String>,
    pub pro: bool,
    /// Connected to the relay: phones can reach this computer from anywhere.
    pub relay: bool,
    pub phones: Vec<PhoneView>,
    pub pairing: Option<PairView>,
    /// A phone scanned the code and waits for the user to allow it.
    pub confirm: Option<ConfirmView>,
    /// The last thing that went wrong (pairing, the server), for the dialog.
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmView {
    pub phone: String,
    pub platform: Option<String>,
    /// The account the phone is signed in to (its email).
    pub account: Option<String>,
}

struct Pairing {
    view: PairView,
    poller: JoinHandle<()>,
}

/// A phone that claimed the code, waiting for the user's yes.
struct Confirm {
    desktop: cloud::Named,
    phone: cloud::Named,
    device_token: Option<String>,
    psk: [u8; 32],
    account: Option<String>,
}

pub struct Remote {
    dir: PathBuf,
    pub hub: Arc<Hub>,
    app: OnceLock<AppHandle>,
    core: OnceLock<Arc<Core>>,
    config: Mutex<Config>,
    server: Mutex<Option<server::Running>>,
    pairing: Mutex<Option<Pairing>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    relay_up: Arc<AtomicBool>,
    error: Mutex<Option<String>>,
    /// Turning Remote on, off and restarting it happen one at a time.
    life: tokio::sync::Mutex<()>,
    /// Bumped by each new pairing and each cancel: a pairing that started before is dropped.
    pair_gen: AtomicU64,
    confirm: Mutex<Option<Confirm>>,
}

/// The computer's name as the user knows it ("Adam's MacBook Pro").
pub fn desktop_name() -> String {
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("scutil").args(["--get", "ComputerName"]).output() {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    hostname::get().map(|h| h.to_string_lossy().trim_end_matches(".local").to_string()).unwrap_or_else(|_| "KlikVibe".into())
}

fn decode_key(psk: &str) -> Option<[u8; 32]> {
    URL_SAFE_NO_PAD.decode(psk).ok()?.try_into().ok()
}

/// What the phone may do, done with the app's own terminals and projects.
struct AppHost(AppHandle);

impl session::Host for AppHost {
    fn hello(&self) -> Value {
        let os = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(windows) {
            "windows"
        } else {
            "linux"
        };
        json!({ "name": desktop_name(), "os": os, "app": self.0.package_info().version.to_string() })
    }

    fn write(&self, id: &str, data: &str) -> Result<(), String> {
        self.0.state::<Terminals>().write(id, data).map_err(|e| e.to_string())
    }

    fn git_status(&self, root: &str) -> Result<Value, String> {
        let root = self.0.state::<Projects>().root(root).map_err(|e| e.to_string())?;
        serde_json::to_value(git::status(&root).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn git_diff(&self, root: &str, path: &str) -> Result<String, String> {
        let root = self.0.state::<Projects>().root(root).map_err(|e| e.to_string())?;
        // Only files Git lists as changed: never a path the phone made up.
        let status = git::status(&root).map_err(|e| e.to_string())?;
        if !status.changes.iter().any(|c| c.path == path) {
            return Err("Not a changed file".into());
        }
        git::diff(&root, path).map_err(|e| e.to_string())
    }
}

impl Remote {
    pub fn new(dir: PathBuf, hub: Arc<Hub>) -> Self {
        let config = store::load(&dir);
        Self {
            dir,
            hub,
            app: OnceLock::new(),
            core: OnceLock::new(),
            config: Mutex::new(config),
            server: Mutex::new(None),
            pairing: Mutex::new(None),
            tasks: Mutex::new(Vec::new()),
            relay_up: Arc::new(AtomicBool::new(false)),
            error: Mutex::new(None),
            life: tokio::sync::Mutex::new(()),
            pair_gen: AtomicU64::new(0),
            confirm: Mutex::new(None),
        }
    }

    fn app(&self) -> &AppHandle {
        self.app.get().expect("Remote::init")
    }

    fn core(&self) -> &Arc<Core> {
        self.core.get().expect("Remote::init")
    }

    fn changed(&self) {
        let _ = self.app().emit(CHANGED, ());
    }

    fn version(&self) -> String {
        self.app().package_info().version.to_string()
    }

    fn save(&self, config: &Config) {
        if let Err(e) = store::save(&self.dir, config) {
            *self.error.lock().unwrap() = Some(e.to_string());
        }
    }

    /// The paired phones' keys, handed to Core.
    fn sync_keys(&self) {
        let keys: HashMap<String, [u8; 32]> =
            self.config.lock().unwrap().phones.iter().filter_map(|p| Some((p.id.clone(), decode_key(&p.psk)?))).collect();
        self.core().set_keys(keys);
    }

    /// Called once at startup; turns Remote on again if it was on.
    pub fn init(app: &AppHandle) {
        let me = app.state::<Remote>();
        let _ = me.app.set(app.clone());
        let handle = app.clone();
        let core = Core::new(me.hub.clone(), Arc::new(AppHost(app.clone())), move || {
            let _ = handle.emit(CHANGED, ());
        });
        let _ = me.core.set(Arc::new(core));
        if me.config.lock().unwrap().enabled {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let me = app.state::<Remote>();
                let _life = me.life.lock().await;
                if me.config.lock().unwrap().enabled {
                    let _ = me.start().await;
                }
            });
        }
    }

    pub fn status(&self) -> Status {
        let c = self.config.lock().unwrap().clone();
        let core = self.core();
        Status {
            enabled: c.enabled,
            port: self.server.lock().unwrap().as_ref().map(|r| r.port),
            addresses: if c.enabled { server::lan_addresses() } else { Vec::new() },
            linked: c.device_token.is_some(),
            account: c.account,
            pro: c.pro,
            relay: self.relay_up.load(Ordering::Relaxed),
            phones: c
                .phones
                .iter()
                .map(|p| PhoneView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    platform: p.platform.clone(),
                    connected: core.is_connected(&p.id),
                })
                .collect(),
            pairing: self.pairing.lock().unwrap().as_ref().map(|p| p.view.clone()),
            confirm: self.confirm.lock().unwrap().as_ref().map(|c| ConfirmView {
                phone: c.phone.name.clone(),
                platform: c.phone.platform.clone(),
                account: c.account.clone(),
            }),
            error: self.error.lock().unwrap().clone(),
        }
    }

    pub async fn set_enabled(&self, on: bool) -> Result<Status, String> {
        let _life = self.life.lock().await;
        {
            let mut c = self.config.lock().unwrap();
            c.enabled = on;
            let snapshot = c.clone();
            drop(c);
            self.save(&snapshot);
        }
        if on {
            self.start().await?;
        } else {
            self.stop();
        }
        self.changed();
        Ok(self.status())
    }

    /// Listens on the LAN, and keeps statuses and the account in sync while on. Called with `life` held.
    async fn start(&self) -> Result<(), String> {
        if self.server.lock().unwrap().is_some() {
            return Ok(());
        }
        self.listen().await?;
        self.sync_keys();
        let app = self.app().clone();
        let tick = tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                app.state::<Remote>().hub.tick();
            }
        });
        let app = self.app().clone();
        let beat = tauri::async_runtime::spawn(async move {
            loop {
                app.state::<Remote>().heartbeat().await;
                tokio::time::sleep(HEARTBEAT).await;
            }
        });
        // With Pro, phones away from this Wi-Fi reach it through the relay.
        let app = self.app().clone();
        let core = self.core().clone();
        let up = self.relay_up.clone();
        let relay = tauri::async_runtime::spawn(async move {
            let me = app.state::<Remote>();
            let token = || {
                let c = me.config.lock().unwrap();
                if c.enabled && c.pro { c.device_token.clone() } else { None }
            };
            // A phone removed in the app: forgotten now, not at the next heartbeat.
            let revoke = |id: &str| me.forget_phone(id);
            relay::run(core, cloud::base(), token, up, || me.changed(), revoke).await;
        });
        self.tasks.lock().unwrap().extend([tick, beat, relay]);
        Ok(())
    }

    /// Called with `life` held.
    fn stop(&self) {
        self.server.lock().unwrap().take();
        for t in self.tasks.lock().unwrap().drain(..) {
            t.abort();
        }
        self.relay_up.store(false, Ordering::Relaxed);
        self.pair_cancel();
        self.confirm.lock().unwrap().take();
        // No key is accepted while off, so connections that are open (or still handshaking) end.
        self.core().set_keys(HashMap::new());
    }

    /// Starts the LAN server, on the port of last time if it is free.
    async fn listen(&self) -> Result<(), String> {
        let (port, desktop_id) = {
            let c = self.config.lock().unwrap();
            (c.port.unwrap_or(server::DEFAULT_PORT), c.desktop_id.clone())
        };
        let running = server::start(self.core().clone(), port, desktop_id.as_deref(), &desktop_name()).await.map_err(|e| e.to_string())?;
        let mut c = self.config.lock().unwrap();
        if c.port != Some(running.port) {
            c.port = Some(running.port);
            let snapshot = c.clone();
            drop(c);
            self.save(&snapshot);
        }
        *self.server.lock().unwrap() = Some(running);
        Ok(())
    }

    /// Announces the computer under its id as it is now (new, or none after unlinking). The LAN
    /// server keeps running on its port: restarting it could land on another port while the old
    /// one is still closing, and phones would no longer find it.
    async fn reannounce(&self) {
        let _life = self.life.lock().await;
        let id = self.config.lock().unwrap().desktop_id.clone();
        if let Some(running) = self.server.lock().unwrap().as_mut() {
            running.reannounce(id.as_deref(), &desktop_name());
        }
    }

    /// Forgets the account: the computer is no longer linked, and its phones' keys go.
    fn forget_account(&self) {
        let mut c = self.config.lock().unwrap();
        c.device_token = None;
        c.desktop_id = None;
        c.account = None;
        c.pro = false;
        c.phones.clear();
        let snapshot = c.clone();
        drop(c);
        self.save(&snapshot);
        self.core().set_keys(HashMap::new());
    }

    /// Syncs with the account: its phones (a removed one's key is forgotten), Pro, the account name.
    async fn heartbeat(&self) {
        let Some(token) = self.config.lock().unwrap().device_token.clone() else { return };
        match cloud::heartbeat(&token, cloud::info(&desktop_name(), &self.version())).await {
            Ok(hb) => {
                let mut c = self.config.lock().unwrap();
                let before = c.clone();
                c.account = hb.account;
                c.pro = hb.pro;
                c.desktop_id = Some(hb.desktop.id);
                c.phones.retain(|p| hb.phones.iter().any(|q| q.id == p.id));
                for p in &mut c.phones {
                    if let Some(q) = hb.phones.iter().find(|q| q.id == p.id) {
                        p.name = q.name.clone();
                        p.platform = q.platform.clone();
                    }
                }
                let snapshot = c.clone();
                drop(c);
                if snapshot != before {
                    self.save(&snapshot);
                    if snapshot.enabled {
                        self.sync_keys();
                    }
                    self.changed();
                }
                *self.error.lock().unwrap() = None;
            }
            Err(cloud::CloudError::Unlinked) => {
                self.forget_account();
                self.changed();
            }
            // Offline: the LAN keeps working with the keys it has.
            Err(e) => eprintln!("remote: heartbeat failed: {e}"),
        }
    }

    /// Shows a QR code for a phone to scan. When one claims it, the user is asked to allow it
    /// (`pair_confirm`); then the computer is linked to the phone's account (the first time)
    /// and the phone's key is kept.
    pub async fn pair_start(&self) -> Result<Status, String> {
        if !self.config.lock().unwrap().enabled {
            self.set_enabled(true).await?;
        }
        self.pair_cancel();
        let generation = self.pair_gen.load(Ordering::SeqCst);
        let token = self.config.lock().unwrap().device_token.clone();
        let started = match cloud::pair_start(cloud::info(&desktop_name(), &self.version()), token.as_deref()).await {
            Ok(s) => s,
            Err(cloud::CloudError::Unlinked) => {
                // Removed from the account meanwhile: start over as a new computer.
                self.forget_account();
                cloud::pair_start(cloud::info(&desktop_name(), &self.version()), None).await.map_err(|e| e.to_string())?
            }
            Err(e) => return Err(e.to_string()),
        };
        // Cancelled (dialog closed, Remote turned off) while the code was being made.
        if self.pair_gen.load(Ordering::SeqCst) != generation {
            return Ok(self.status());
        }
        let psk = random_key()?;
        let port = self.server.lock().unwrap().as_ref().map(|r| r.port);
        let payload = json!({
            "v": 1,
            "c": started.code,
            "k": URL_SAFE_NO_PAD.encode(psk),
            "n": desktop_name(),
            "a": server::lan_addresses(),
            "p": port,
        });
        let url = format!("{}/link#{}", cloud::base(), URL_SAFE_NO_PAD.encode(payload.to_string()));
        let svg = qr_svg(&url)?;
        let app = self.app().clone();
        let poll = started.poll_token.clone();
        let poller = tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(PAIR_POLL).await;
                let me = app.state::<Remote>();
                if me.pair_gen.load(Ordering::SeqCst) != generation {
                    break;
                }
                match cloud::pair_status(&poll).await {
                    Ok(cloud::PairStatus::Pending) => continue,
                    Ok(cloud::PairStatus::Linked { desktop, phone, device_token, account }) => {
                        me.pairing.lock().unwrap().take();
                        *me.confirm.lock().unwrap() = Some(Confirm { desktop, phone, device_token, psk, account });
                        me.changed();
                        break;
                    }
                    Err(cloud::CloudError::Other(_)) => continue,
                    Err(e) => {
                        *me.error.lock().unwrap() = Some(e.to_string());
                        me.pairing.lock().unwrap().take();
                        me.changed();
                        break;
                    }
                }
            }
        });
        let old = self.pairing.lock().unwrap().replace(Pairing { view: PairView { url, svg, expires_at: started.expires_at }, poller });
        if let Some(old) = old {
            old.poller.abort();
        }
        *self.error.lock().unwrap() = None;
        self.changed();
        Ok(self.status())
    }

    /// The user's answer to a phone that scanned the code. No: a computer it had just added to
    /// its account is removed from that account again, and its key is never kept.
    pub async fn pair_confirm(&self, allow: bool) -> Status {
        let Some(c) = self.confirm.lock().unwrap().take() else { return self.status() };
        if !allow {
            if let Some(t) = c.device_token {
                let _ = cloud::unlink(&t).await;
            }
            self.changed();
            return self.status();
        }
        let new_id = {
            let mut cfg = self.config.lock().unwrap();
            let new_id = cfg.desktop_id.as_deref() != Some(c.desktop.id.as_str());
            if let Some(t) = c.device_token {
                cfg.device_token = Some(t);
            }
            cfg.desktop_id = Some(c.desktop.id);
            cfg.phones.retain(|p| p.id != c.phone.id);
            cfg.phones.push(Phone { id: c.phone.id, name: c.phone.name, platform: c.phone.platform, psk: URL_SAFE_NO_PAD.encode(c.psk) });
            let snapshot = cfg.clone();
            drop(cfg);
            self.save(&snapshot);
            new_id
        };
        if self.config.lock().unwrap().enabled {
            self.sync_keys();
        }
        if new_id {
            self.reannounce().await;
        }
        self.changed();
        self.heartbeat().await;
        self.status()
    }

    pub fn pair_cancel(&self) {
        self.pair_gen.fetch_add(1, Ordering::SeqCst);
        if let Some(p) = self.pairing.lock().unwrap().take() {
            p.poller.abort();
        }
    }

    /// Forgets a phone on this computer only (it stays in the account until removed in the app).
    pub fn forget_phone(&self, id: &str) {
        let mut c = self.config.lock().unwrap();
        if !c.phones.iter().any(|p| p.id == id) {
            return;
        }
        c.phones.retain(|p| p.id != id);
        let snapshot = c.clone();
        drop(c);
        self.save(&snapshot);
        if snapshot.enabled {
            self.sync_keys();
        }
        self.changed();
    }

    /// "Disconnect from account": the server forgets the computer, and it forgets every phone.
    pub async fn unlink(&self) {
        let token = self.config.lock().unwrap().device_token.clone();
        if let Some(t) = token {
            let _ = cloud::unlink(&t).await;
        }
        self.forget_account();
        self.reannounce().await;
        self.changed();
    }

    /// An agent needs the user: the connected phones are told (end-to-end encrypted), and the
    /// account's phones get a push (Pro). The push carries only the title (agent and project),
    /// never what the agent said: it passes through the KlikVibe server, Apple and Google.
    pub fn attention(&self, root: &str, term: &str, title: &str, body: &str) {
        self.hub.attention(root, term, title, body);
        let (token, pro, enabled) = {
            let c = self.config.lock().unwrap();
            (c.device_token.clone(), c.pro, c.enabled)
        };
        if let (Some(token), true, true) = (token, pro, enabled) {
            let title = title.to_string();
            tauri::async_runtime::spawn(async move {
                let _ = cloud::notify(&token, &title, "").await;
            });
        }
    }
}

/// A new phone key from the operating system's random generator.
fn random_key() -> Result<[u8; 32], String> {
    let mut k = [0u8; 32];
    getrandom::fill(&mut k).map_err(|e| e.to_string())?;
    Ok(k)
}

fn qr_svg(text: &str) -> Result<String, String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::L).map_err(|e| e.to_string())?;
    Ok(code.render::<svg::Color>().min_dimensions(220, 220).dark_color(svg::Color("#000000")).light_color(svg::Color("#ffffff")).build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        let k = random_key().unwrap();
        assert_eq!(decode_key(&URL_SAFE_NO_PAD.encode(k)), Some(k));
        assert_eq!(decode_key("short"), None);
        assert_ne!(random_key().unwrap(), random_key().unwrap());
    }

    #[test]
    fn the_qr_code_fits_a_pairing_url() {
        let payload = json!({ "v": 1, "c": "ABCDE12345", "k": URL_SAFE_NO_PAD.encode([0u8; 32]), "n": "Adam's MacBook Pro", "a": ["192.168.100.23", "100.101.102.103"], "p": 47615 });
        let url = format!("{}/link#{}", cloud::SERVER, URL_SAFE_NO_PAD.encode(payload.to_string()));
        assert!(qr_svg(&url).unwrap().starts_with("<?xml"));
    }
}
