//! One phone's conversation with the desktop, after the handshake: the JSON
//! messages of PROTOCOL.md. It only knows plaintext; server.rs and relay.rs
//! carry it (encrypted) over the LAN or through the relay.

use super::hub::{Event, Hub};
use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

pub const VERSION: u32 = 1;
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
}

struct Session {
    hub: Arc<Hub>,
    host: Arc<dyn Host>,
    tx: mpsc::Sender<Vec<u8>>,
    /// The terminals this phone follows, and the output offset it has seen up to.
    open: HashMap<String, u64>,
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
            "ping" => self.send(json!({ "t": "pong" })).await,
            // Newer phones may send what this version doesn't know.
            _ => true,
        }
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
    let mut s = Session { hub, host, tx, open: HashMap::new() };
    let mut hello = s.host.hello();
    hello["t"] = json!("hello");
    hello["v"] = json!(VERSION);
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
    use crate::remote::hub::{ProjectMeta, TerminalMeta};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake(Mutex<Vec<(String, String)>>);

    impl Host for Fake {
        fn hello(&self) -> Value {
            json!({ "name": "Mac", "os": "macos", "app": "0.7.0" })
        }
        fn write(&self, id: &str, data: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((id.into(), data.into()));
            Ok(())
        }
        fn git_status(&self, _root: &str) -> Result<Value, String> {
            Ok(json!({ "repo": true, "branch": "main", "changes": [] }))
        }
        fn git_diff(&self, _root: &str, path: &str) -> Result<String, String> {
            Ok(if path == "big" { "é".repeat(MAX_DIFF) } else { format!("diff {path}") })
        }
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
        assert_eq!(host.0.lock().unwrap().as_slice(), [("t1".to_string(), "y".to_string())]);

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
}
