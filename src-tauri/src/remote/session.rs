//! One phone's conversation with the desktop, after the handshake: the JSON
//! messages of PROTOCOL.md. It only knows plaintext; server.rs and relay.rs
//! carry it (encrypted) over the LAN or through the relay.

use super::hub::{Event, Hub};
use super::upload::{self, Chunk, Uploads};
use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

pub const VERSION: u32 = 1;
/// What this desktop can do beyond the first version, in `hello`: a phone offers only these.
const FEATURES: [&str; 2] = ["term.new", "file.put"];
/// How long the window has to open a terminal for `term.new`.
const TERM_NEW_WAIT: Duration = Duration::from_secs(10);
/// Raw output per `term.data` message (base64 makes it a third larger; it still fits a Noise message).
const DATA_CHUNK: usize = 32 * 1024;
/// A diff larger than this is cut (a Noise message holds 64 KiB; JSON escaping can add to it, see `fit`).
const MAX_DIFF: usize = 48 * 1024;
/// Changed files listed at most in a `git.status` answer.
const MAX_CHANGES: usize = 300;
/// The largest JSON message that fits a Noise message.
const MAX_MESSAGE: usize = super::noise::MAX_PLAINTEXT;

/// What a session needs from the rest of the app.
pub trait Host: Send + Sync + 'static {
    /// `name`, `os` and `app` for the `hello` answer.
    fn hello(&self) -> Value;
    /// Types into a terminal.
    fn write(&self, id: &str, data: &str) -> Result<(), String>;
    /// The project's `git status` (git::Status as JSON).
    fn git_status(&self, root: &str) -> Result<Value, String>;
    /// The working tree's diff of a changed file.
    fn git_diff(&self, root: &str, path: &str) -> Result<String, String>;
    /// Asks the window for a new terminal tab in an open project, running `agent` (an id the
    /// window reported, see `Hub::agents`) or a plain shell. The new terminal's id comes later.
    fn term_new(&self, root: &str, agent: Option<&str>) -> Result<oneshot::Receiver<Result<String, String>>, String>;
    /// Where `file.put` saves images.
    fn upload_dir(&self) -> PathBuf {
        upload::default_dir()
    }
}

struct Session {
    hub: Arc<Hub>,
    host: Arc<dyn Host>,
    tx: mpsc::Sender<Vec<u8>>,
    /// The terminals this phone follows, and the output offset it has seen up to.
    open: HashMap<String, u64>,
    /// A `term.new` is waiting for the window: one at a time.
    opening: Arc<AtomicBool>,
    uploads: Uploads,
}

impl Session {
    /// Sends one message. One too large for a Noise message becomes an error for its request,
    /// so the phone isn't left waiting.
    async fn send(&self, v: Value) -> bool {
        let mut text = v.to_string();
        if text.len() > MAX_MESSAGE {
            text = json!({ "t": "error", "req": v.get("req").cloned().unwrap_or(Value::Null), "message": "Too large to send" }).to_string();
        }
        self.tx.send(text.into_bytes()).await.is_ok()
    }

    async fn data(&self, id: &str, bytes: &[u8]) -> bool {
        for chunk in bytes.chunks(DATA_CHUNK) {
            let data = base64::engine::general_purpose::STANDARD.encode(chunk);
            if !self.send(json!({ "t": "term.data", "id": id, "data": data })).await {
                return false;
            }
        }
        true
    }

    /// `term.reset`, then everything the terminal printed so far.
    async fn open_term(&mut self, id: &str) -> bool {
        let Some(o) = self.hub.root_of(id).and_then(|_| self.hub.open(id)) else {
            // With the terminal's id, so the phone shows it on that terminal's screen.
            return self.send(json!({ "t": "error", "id": id, "message": "No such terminal" })).await;
        };
        self.open.insert(id.to_string(), o.end);
        self.send(json!({ "t": "term.reset", "id": id, "cols": o.cols, "rows": o.rows })).await
            && self.data(id, &o.scrollback).await
            && match o.exited {
                Some(code) => self.send(json!({ "t": "term.exit", "id": id, "code": code })).await,
                None => true,
            }
    }

    async fn projects(&self) -> bool {
        self.send(json!({ "t": "projects", "projects": self.hub.projects() })).await
    }

    async fn event(&mut self, e: Event) -> bool {
        match e {
            Event::Data { id, offset, bytes } => {
                // Output the scrollback already had (queued before the phone opened the terminal) is skipped.
                let Some(seen) = self.open.get_mut(&id) else { return true };
                let end = offset + bytes.len() as u64;
                if end <= *seen {
                    return true;
                }
                let skip = seen.saturating_sub(offset) as usize;
                *seen = end;
                self.data(&id, &bytes[skip..]).await
            }
            Event::Exit { id, code } if self.open.contains_key(&id) => self.send(json!({ "t": "term.exit", "id": id, "code": code })).await,
            Event::Projects => self.projects().await,
            Event::Attention { root, term, title, body } => {
                self.send(json!({ "t": "attention", "root": root, "term": term, "title": title, "body": body })).await
            }
            _ => true,
        }
    }

    async fn git<T: Send + 'static>(&self, f: impl FnOnce(Arc<dyn Host>) -> Result<T, String> + Send + 'static) -> Result<T, String> {
        let host = self.host.clone();
        tokio::task::spawn_blocking(move || f(host)).await.map_err(|e| e.to_string())?
    }

    /// One message from the phone; false ends the session.
    async fn message(&mut self, m: Value) -> bool {
        let req = m.get("req").cloned().unwrap_or(Value::Null);
        let s = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        match m.get("t").and_then(Value::as_str).unwrap_or_default() {
            "projects" => self.projects().await,
            "term.open" => self.open_term(&s("id")).await,
            "term.close" => {
                self.open.remove(&s("id"));
                true
            }
            "term.input" => {
                let id = s("id");
                let id_for_error = id.clone();
                // Only terminals of the open projects, the ones the phone was shown.
                // Off the async workers: a program that isn't reading its input can block the write.
                let result = match self.hub.root_of(&id) {
                    Some(_) => {
                        let data = s("data");
                        self.git(move |h| h.write(&id, &data)).await
                    }
                    None => Err("No such terminal".into()),
                };
                match result {
                    Ok(()) => true,
                    Err(e) => self.send(json!({ "t": "error", "id": id_for_error, "message": e })).await,
                }
            }
            "git.status" => {
                let root = s("root");
                let r =
                    if self.hub.has_project(&root) { self.git(move |h| h.git_status(&root)).await } else { Err("Project not open".into()) };
                match r {
                    Ok(mut status) => {
                        // A huge working tree (an unignored build folder) is listed only in part.
                        let mut truncated = false;
                        if let Some(list) = status.get_mut("changes").and_then(Value::as_array_mut)
                            && list.len() > MAX_CHANGES
                        {
                            list.truncate(MAX_CHANGES);
                            truncated = true;
                        }
                        self.send(json!({ "t": "git.status", "req": req, "status": status, "truncated": truncated })).await
                    }
                    Err(e) => self.send(json!({ "t": "error", "req": req, "message": e })).await,
                }
            }
            "git.diff" => {
                let (root, path) = (s("root"), s("path"));
                let r = if self.hub.has_project(&root) {
                    self.git(move |h| h.git_diff(&root, &path)).await
                } else {
                    Err("Project not open".into())
                };
                match r {
                    Ok(diff) => {
                        let (diff, cut) = fit(diff);
                        self.send(json!({ "t": "git.diff", "req": req, "diff": diff, "truncated": cut })).await
                    }
                    Err(e) => self.send(json!({ "t": "error", "req": req, "message": e })).await,
                }
            }
            "term.new" => {
                let root = s("root");
                match self.term_new(&root, m.get("agent")) {
                    Ok(answer) => {
                        // Answered from its own task: the phone's terminals keep streaming meanwhile.
                        let (tx, opening) = (self.tx.clone(), self.opening.clone());
                        tokio::spawn(async move {
                            let r = match tokio::time::timeout(TERM_NEW_WAIT, answer).await {
                                Ok(Ok(r)) => r,
                                Ok(Err(_)) | Err(_) => Err("The terminal did not start".into()),
                            };
                            opening.store(false, Ordering::SeqCst);
                            let reply = match r {
                                Ok(id) => json!({ "t": "term.new", "req": req, "id": id }),
                                Err(e) => json!({ "t": "error", "req": req, "message": e }),
                            };
                            let _ = tx.send(reply.to_string().into_bytes()).await;
                        });
                        true
                    }
                    Err(e) => self.send(json!({ "t": "error", "req": req, "message": e })).await,
                }
            }
            "file.put" => {
                let n = |k: &str| m.get(k).and_then(Value::as_u64);
                let chunk = Chunk {
                    up: s("up"),
                    name: s("name"),
                    size: n("size").unwrap_or(0),
                    // A chunk without its offset can't be placed: it matches no upload.
                    offset: n("offset").unwrap_or(u64::MAX),
                    data: s("data"),
                    last: m.get("last").and_then(Value::as_bool).unwrap_or(false),
                };
                match self.uploads.put(chunk).await {
                    Ok(None) => self.send(json!({ "t": "file.put", "req": req, "ok": true })).await,
                    Ok(Some(path)) => self.send(json!({ "t": "file.put", "req": req, "ok": true, "path": path.to_string_lossy() })).await,
                    Err(e) => self.send(json!({ "t": "error", "req": req, "message": e })).await,
                }
            }
            "ping" => self.send(json!({ "t": "pong" })).await,
            // Newer phones may send what this version doesn't know.
            _ => true,
        }
    }
}

impl Session {
    /// Checks a `term.new` and hands it to the window. Only an agent the window listed runs:
    /// never a command from the phone.
    fn term_new(&self, root: &str, agent: Option<&Value>) -> Result<oneshot::Receiver<Result<String, String>>, String> {
        let agent = match agent {
            None | Some(Value::Null) => None,
            Some(Value::String(a)) if self.hub.agents().iter().any(|x| &x.id == a) => Some(a.as_str()),
            Some(_) => return Err("Unknown agent".into()),
        };
        if !self.hub.has_project(root) {
            return Err("Project not open".into());
        }
        if self.opening.swap(true, Ordering::SeqCst) {
            return Err("Already opening a terminal".into());
        }
        self.host.term_new(root, agent).inspect_err(|_| self.opening.store(false, Ordering::SeqCst))
    }
}

/// Cuts a diff so its JSON (where escaping can double control characters) fits a message.
fn fit(mut diff: String) -> (String, bool) {
    fn cut_at(s: &mut String, mut end: usize) {
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    let json_len = |s: &str| serde_json::to_string(s).map(|j| j.len()).unwrap_or(usize::MAX);
    let mut cut = false;
    if diff.len() > MAX_DIFF {
        cut_at(&mut diff, MAX_DIFF);
        cut = true;
    }
    while json_len(&diff) > MAX_DIFF + 4096 {
        let end = diff.len() * 3 / 4;
        cut_at(&mut diff, end);
        cut = true;
    }
    (diff, cut)
}

/// Runs until the phone hangs up (`rx` ends) or the connection can't be written.
/// The first message must be `hello`.
pub async fn serve(hub: Arc<Hub>, host: Arc<dyn Host>, mut rx: mpsc::Receiver<Vec<u8>>, tx: mpsc::Sender<Vec<u8>>) {
    let parse = |b: Vec<u8>| serde_json::from_slice::<Value>(&b).ok();
    let Some(first) = rx.recv().await.and_then(parse) else { return };
    if first.get("t").and_then(Value::as_str) != Some("hello") {
        let _ = tx.send(json!({ "t": "error", "message": "Expected hello" }).to_string().into_bytes()).await;
        return;
    }
    let mut events = hub.subscribe();
    let uploads = Uploads::new(host.upload_dir());
    let mut s = Session { hub, host, tx, open: HashMap::new(), opening: Arc::default(), uploads };
    let mut hello = s.host.hello();
    hello["t"] = json!("hello");
    hello["v"] = json!(VERSION);
    hello["features"] = json!(FEATURES);
    hello["agents"] = json!(s.hub.agents());
    if !s.send(hello).await || !s.projects().await {
        return;
    }
    loop {
        let ok = tokio::select! {
            m = rx.recv() => match m {
                Some(bytes) => match parse(bytes) {
                    Some(m) => s.message(m).await,
                    None => true,
                },
                None => false,
            },
            e = events.recv() => match e {
                Ok(e) => s.event(e).await,
                // Too far behind (a slow connection): start the open terminals over.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let ids: Vec<String> = s.open.drain().map(|(id, _)| id).collect();
                    let mut ok = true;
                    for id in ids {
                        ok = ok && s.open_term(&id).await;
                    }
                    ok
                }
                Err(broadcast::error::RecvError::Closed) => false,
            },
        };
        if !ok {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::hub::{AgentMeta, ProjectMeta, TerminalMeta};
    use std::sync::Mutex;

    struct Fake {
        typed: Mutex<Vec<(String, String)>>,
        /// Where this test's uploads go.
        dir: PathBuf,
    }

    impl Default for Fake {
        fn default() -> Self {
            Self { typed: Mutex::default(), dir: std::env::temp_dir().join(format!("klikvibe-remote-test-{}", uuid::Uuid::new_v4())) }
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    impl Host for Fake {
        fn hello(&self) -> Value {
            json!({ "name": "Mac", "os": "macos", "app": "0.7.0" })
        }
        fn write(&self, id: &str, data: &str) -> Result<(), String> {
            self.typed.lock().unwrap().push((id.into(), data.into()));
            Ok(())
        }
        fn git_status(&self, _root: &str) -> Result<Value, String> {
            Ok(json!({ "repo": true, "branch": "main", "changes": [] }))
        }
        fn git_diff(&self, _root: &str, path: &str) -> Result<String, String> {
            Ok(if path == "big" { "é".repeat(MAX_DIFF) } else { format!("diff {path}") })
        }
        fn term_new(&self, root: &str, agent: Option<&str>) -> Result<oneshot::Receiver<Result<String, String>>, String> {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Ok(format!("new {root} {}", agent.unwrap_or("shell"))));
            Ok(rx)
        }
        fn upload_dir(&self) -> PathBuf {
            self.dir.clone()
        }
    }

    /// A session past its hello and first project list.
    async fn connected(hub: Arc<Hub>, host: Arc<Fake>) -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>, tokio::task::JoinHandle<()>) {
        let (to_desktop, rx) = mpsc::channel(16);
        let (tx, mut from_desktop) = mpsc::channel(64);
        let task = tokio::spawn(serve(hub, host, rx, tx));
        to_desktop.send(br#"{"t":"hello","v":1}"#.to_vec()).await.unwrap();
        next(&mut from_desktop).await; // hello
        next(&mut from_desktop).await; // projects
        (to_desktop, from_desktop, task)
    }

    fn app_hub() -> Arc<Hub> {
        let hub = Arc::new(Hub::default());
        hub.set_projects(vec![ProjectMeta { root: "/app".into(), name: "app".into(), attention: false, terminals: vec![] }]);
        hub
    }

    fn chunk(req: u64, up: &str, name: &str, size: usize, offset: usize, data: &[u8], last: bool) -> Vec<u8> {
        let data = base64::engine::general_purpose::STANDARD.encode(data);
        json!({ "t": "file.put", "req": req, "up": up, "name": name, "size": size, "offset": offset, "data": data, "last": last })
            .to_string()
            .into_bytes()
    }

    fn files_in(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
    }

    async fn next(rx: &mut mpsc::Receiver<Vec<u8>>) -> Value {
        let b = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn a_session() {
        let hub = Arc::new(Hub::default());
        hub.started("t1", 100, 30);
        hub.started("t2", 80, 24);
        hub.output("t1", b"before");
        hub.set_projects(vec![ProjectMeta {
            root: "/app".into(),
            name: "app".into(),
            attention: false,
            terminals: vec![TerminalMeta { id: "t1".into(), name: "Claude".into(), agent: Some("claude".into()) }],
        }]);
        let host = Arc::new(Fake::default());
        let (to_desktop, rx) = mpsc::channel(16);
        let (tx, mut from_desktop) = mpsc::channel(64);
        let task = tokio::spawn(serve(hub.clone(), host.clone(), rx, tx));
        let say = |v: Value| to_desktop.send(v.to_string().into_bytes());

        say(json!({ "t": "hello", "v": 1, "name": "iPhone" })).await.unwrap();
        let h = next(&mut from_desktop).await;
        assert_eq!((h["t"].as_str(), h["name"].as_str(), h["v"].as_u64()), (Some("hello"), Some("Mac"), Some(1)));
        assert_eq!(h["features"], json!(["term.new", "file.put"]));
        assert_eq!(h["agents"], json!([]));
        let p = next(&mut from_desktop).await;
        assert_eq!(p["projects"][0]["terminals"][0]["id"], "t1");

        say(json!({ "t": "term.open", "id": "t1" })).await.unwrap();
        let r = next(&mut from_desktop).await;
        assert_eq!((r["t"].as_str(), r["cols"].as_u64()), (Some("term.reset"), Some(100)));
        let d = next(&mut from_desktop).await;
        assert_eq!(d["data"], base64::engine::general_purpose::STANDARD.encode("before"));
        hub.output("t1", b"live");
        hub.output("t2", b"not shown");
        let d = next(&mut from_desktop).await;
        assert_eq!(d["data"], base64::engine::general_purpose::STANDARD.encode("live"));

        // A terminal of no open project can't be opened or typed into.
        say(json!({ "t": "term.open", "id": "t2" })).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "error");
        say(json!({ "t": "term.input", "id": "t2", "data": "rm -rf /\r" })).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "error");
        say(json!({ "t": "term.input", "id": "t1", "data": "y" })).await.unwrap();

        say(json!({ "t": "git.status", "req": 1, "root": "/app" })).await.unwrap();
        let g = next(&mut from_desktop).await;
        assert_eq!((g["req"].as_u64(), g["status"]["branch"].as_str()), (Some(1), Some("main")));
        say(json!({ "t": "git.status", "req": 2, "root": "/etc" })).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "error");
        say(json!({ "t": "git.diff", "req": 3, "root": "/app", "path": "big" })).await.unwrap();
        let g = next(&mut from_desktop).await;
        assert_eq!(g["truncated"], true);
        assert!(g["diff"].as_str().unwrap().len() <= MAX_DIFF);

        say(json!({ "t": "ping" })).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "pong");
        assert_eq!(host.typed.lock().unwrap().as_slice(), [("t1".to_string(), "y".to_string())]);

        hub.attention("/app", "t1", "Claude · app", "Waiting");
        assert_eq!(next(&mut from_desktop).await["t"], "attention");

        drop(to_desktop);
        task.await.unwrap();
    }

    #[test]
    fn diffs_fit_a_message() {
        let (d, cut) = fit("a".repeat(10));
        assert_eq!((d.len(), cut), (10, false));
        // Control characters double (or worse) in JSON: still cut to fit.
        let (d, cut) = fit("\u{1b}".repeat(MAX_DIFF));
        assert!(cut && serde_json::to_string(&d).unwrap().len() <= MAX_DIFF + 4096);
        let (d, cut) = fit("é".repeat(MAX_DIFF));
        assert!(cut && d.len() <= MAX_DIFF);
    }

    #[tokio::test]
    async fn output_is_not_sent_twice() {
        let hub = Arc::new(Hub::default());
        hub.started("t1", 80, 24);
        hub.set_projects(vec![ProjectMeta {
            root: "/app".into(),
            name: "app".into(),
            attention: false,
            terminals: vec![TerminalMeta { id: "t1".into(), name: "zsh".into(), agent: None }],
        }]);
        let (to_desktop, rx) = mpsc::channel(16);
        let (tx, mut from_desktop) = mpsc::channel(64);
        let task = tokio::spawn(serve(hub.clone(), Arc::new(Fake::default()), rx, tx));
        to_desktop.send(br#"{"t":"hello","v":1}"#.to_vec()).await.unwrap();
        next(&mut from_desktop).await; // hello
        next(&mut from_desktop).await; // projects
        // Printed while the session is subscribed but before the phone opens the terminal:
        // it is in the scrollback, and must not arrive again as live output.
        hub.output("t1", b"early");
        to_desktop.send(br#"{"t":"term.open","id":"t1"}"#.to_vec()).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "term.reset");
        assert_eq!(next(&mut from_desktop).await["data"], base64::engine::general_purpose::STANDARD.encode("early"));
        hub.output("t1", b"late");
        assert_eq!(next(&mut from_desktop).await["data"], base64::engine::general_purpose::STANDARD.encode("late"));
        drop(to_desktop);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn hello_comes_first() {
        let (to_desktop, rx) = mpsc::channel(4);
        let (tx, mut from_desktop) = mpsc::channel(4);
        let task = tokio::spawn(serve(Arc::new(Hub::default()), Arc::new(Fake::default()), rx, tx));
        to_desktop.send(br#"{"t":"projects"}"#.to_vec()).await.unwrap();
        assert_eq!(next(&mut from_desktop).await["t"], "error");
        task.await.unwrap();
    }

    #[tokio::test]
    async fn new_terminals() {
        let hub = app_hub();
        hub.set_agents(vec![AgentMeta { id: "claude".into(), label: "Claude Code".into() }]);
        let (to, mut from, task) = connected(hub, Arc::new(Fake::default())).await;
        let say = |v: Value| to.send(v.to_string().into_bytes());
        say(json!({ "t": "term.new", "req": 1, "root": "/app", "agent": "claude" })).await.unwrap();
        assert_eq!(next(&mut from).await, json!({ "t": "term.new", "req": 1, "id": "new /app claude" }));
        say(json!({ "t": "term.new", "req": 2, "root": "/app", "agent": null })).await.unwrap();
        assert_eq!(next(&mut from).await["id"], "new /app shell");
        // Never a command of the phone's choosing, nor a project that isn't open.
        say(json!({ "t": "term.new", "req": 3, "root": "/app", "agent": "rm -rf ~" })).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        say(json!({ "t": "term.new", "req": 4, "root": "/etc", "agent": null })).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        drop(to);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn an_upload_in_two_chunks() {
        let host = Arc::new(Fake::default());
        let (to, mut from, task) = connected(app_hub(), host.clone()).await;
        let image: Vec<u8> = (0..upload::MAX_CHUNK + 1000).map(|i| (i % 251) as u8).collect();
        let (a, b) = image.split_at(upload::MAX_CHUNK);
        to.send(chunk(1, "up-12345", "My photo.JPG", image.len(), 0, a, false)).await.unwrap();
        assert_eq!(next(&mut from).await, json!({ "t": "file.put", "req": 1, "ok": true }));
        to.send(chunk(2, "up-12345", "My photo.JPG", image.len(), a.len(), b, true)).await.unwrap();
        let done = next(&mut from).await;
        assert_eq!((done["req"].as_u64(), done["ok"].as_bool()), (Some(2), Some(true)));
        let path = PathBuf::from(done["path"].as_str().unwrap());
        assert_eq!(path.parent(), Some(host.dir.as_path()));
        assert!(path.file_name().unwrap().to_str().unwrap().ends_with("-My_photo.jpg"));
        assert_eq!(std::fs::read(&path).unwrap(), image);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(std::fs::metadata(&host.dir).unwrap().permissions().mode() & 0o777, 0o700);
        }
        // A finished upload stays when the phone hangs up.
        drop(to);
        task.await.unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn a_chunk_out_of_order_ends_the_upload() {
        let host = Arc::new(Fake::default());
        let (to, mut from, task) = connected(app_hub(), host.clone()).await;
        to.send(chunk(1, "up-12345", "a.png", 10, 0, b"12345", false)).await.unwrap();
        assert_eq!(next(&mut from).await["ok"], true);
        assert_eq!(files_in(&host.dir), 1);
        to.send(chunk(2, "up-12345", "a.png", 10, 3, b"45678", true)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        assert_eq!(files_in(&host.dir), 0);
        // It is gone: what follows matches no upload.
        to.send(chunk(3, "up-12345", "a.png", 10, 5, b"67890", true)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        drop(to);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn only_images_of_a_sane_size() {
        let host = Arc::new(Fake::default());
        let (to, mut from, task) = connected(app_hub(), host.clone()).await;
        to.send(chunk(1, "up-12345", "run.sh", 4, 0, b"echo", true)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        to.send(chunk(2, "up-12345", "huge.png", upload::MAX_SIZE as usize + 1, 0, b"x", false)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        to.send(chunk(3, "short", "a.png", 1, 0, b"x", true)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        assert_eq!(files_in(&host.dir), 0);
        // At most two at once; the partial files go when the phone hangs up.
        for (i, up) in ["up-aaaaaa", "up-bbbbbb"].into_iter().enumerate() {
            to.send(chunk(4 + i as u64, up, "a.png", 10, 0, b"12345", false)).await.unwrap();
            assert_eq!(next(&mut from).await["ok"], true);
        }
        to.send(chunk(6, "up-cccccc", "a.png", 10, 0, b"12345", false)).await.unwrap();
        assert_eq!(next(&mut from).await["t"], "error");
        assert_eq!(files_in(&host.dir), 2);
        drop(to);
        task.await.unwrap();
        for _ in 0..50 {
            if files_in(&host.dir) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(files_in(&host.dir), 0);
    }
}
