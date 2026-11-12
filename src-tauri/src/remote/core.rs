//! A phone's connection, whichever way it came (the LAN or the relay): the
//! handshake with the key of the phone it says it is, then its session with
//! every message sealed. Knows nothing about Tauri, so the tests run it whole.

use super::hub::Hub;
use super::noise::{self, Channel};
use super::session::{self, Host};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

/// How long a phone has for its handshake and first message.
/// How long a session's last message may take to go out once the session is over.
const LAST_WORDS: std::time::Duration = std::time::Duration::from_secs(2);
const HANDSHAKE: std::time::Duration = std::time::Duration::from_secs(10);

pub struct Core {
    pub hub: Arc<Hub>,
    pub host: Arc<dyn Host>,
    /// Paired phones: id → key.
    keys: Mutex<HashMap<String, [u8; 32]>>,
    /// Keys handed to phones (`key.new`) that they haven't confirmed yet: id → key. Also accepted
    /// at the handshake, in case the `key.ok` was lost.
    offered: Mutex<HashMap<String, [u8; 32]>>,
    /// Phones connected right now (id → connections), for the Remote dialog.
    connected: Mutex<HashMap<String, usize>>,
    /// A phone that was removed: its connections end.
    kicks: broadcast::Sender<String>,
    /// Called when the connected phones change.
    changed: Box<dyn Fn() + Send + Sync>,
}

/// Decrements the count when a connection ends, however it ends.
struct Connected<'a>(&'a Core, String);

impl Drop for Connected<'_> {
    fn drop(&mut self) {
        let mut map = self.0.connected.lock().unwrap();
        if let Some(n) = map.get_mut(&self.1) {
            *n -= 1;
            if *n == 0 {
                map.remove(&self.1);
            }
        }
        drop(map);
        (self.0.changed)();
    }
}

impl Core {
    pub fn new(hub: Arc<Hub>, host: Arc<dyn Host>, changed: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            hub,
            host,
            keys: Mutex::default(),
            offered: Mutex::default(),
            connected: Mutex::default(),
            kicks: broadcast::channel(16).0,
            changed: Box::new(changed),
        }
    }

    /// Replaces the paired phones. The connections of phones no longer there end, and so do those
    /// made with a key that changed (the same phone added again: its old key is no good any more).
    pub fn set_keys(&self, keys: HashMap<String, [u8; 32]>) {
        let mut cur = self.keys.lock().unwrap();
        let gone: Vec<String> = cur.iter().filter(|(id, k)| keys.get(*id) != Some(*k)).map(|(id, _)| id.clone()).collect();
        *cur = keys;
        drop(cur);
        for id in gone {
            let _ = self.kicks.send(id);
        }
    }

    pub fn is_connected(&self, phone: &str) -> bool {
        self.connected.lock().unwrap().contains_key(phone)
    }

    /// Whether `psk` is still this phone's key.
    fn key_is(&self, phone: &str, psk: &[u8; 32]) -> bool {
        self.keys.lock().unwrap().get(phone) == Some(psk)
    }

    /// Serves one phone: `frames` are the binary frames it sends, `out` the ones
    /// to send it. Returns when either side is done; dropping `out` hangs up.
    pub async fn phone(&self, mut frames: mpsc::Receiver<Vec<u8>>, out: mpsc::Sender<Vec<u8>>) {
        // Subscribed before the key is looked up, so a removal at any point after is seen.
        let mut kicks = self.kicks.subscribe();
        // The first frame: the phone's id (16 bytes), then Noise message 1. A client that
        // sends nothing (or nothing valid) in time is dropped instead of holding a socket.
        let Ok(Some(first)) = tokio::time::timeout(HANDSHAKE, frames.recv()).await else { return };
        if first.len() < 16 {
            return;
        }
        let Ok(id) = uuid::Uuid::from_slice(&first[..16]) else { return };
        let id = id.to_string();
        let Some(current) = self.keys.lock().unwrap().get(&id).copied() else { return };
        let offered = self.offered.lock().unwrap().get(&id).copied();
        // The key, or one this phone was offered but whose confirmation never arrived.
        let (psk, answer, channel) = match noise::respond(&current, &first[16..]) {
            Ok((a, c)) => (current, a, c),
            Err(_) => match offered.map(|k| (k, noise::respond(&k, &first[16..]))) {
                Some((k, Ok((a, c)))) => (k, a, c),
                _ => return,
            },
        };
        if out.send(answer).await.is_err() {
            return;
        }
        let mut channel = channel;
        // Its first sealed message proves it holds the key (a replayed first frame can't make one).
        let Ok(Some(frame)) = tokio::time::timeout(HANDSHAKE, frames.recv()).await else { return };
        let Ok(hello) = channel.open(&frame) else { return };
        if psk != current {
            self.adopt(&id, psk);
        }
        if !self.key_is(&id, &psk) {
            return;
        }
        // This connection's key: it changes when the phone takes a new one.
        let psk = Mutex::new(psk);
        *self.connected.lock().unwrap().entry(id.clone()).or_default() += 1;
        let _guard = Connected(self, id.clone());
        (self.changed)();

        let channel = Arc::new(Mutex::new(channel));
        let (plain_in, session_rx) = mpsc::channel::<Vec<u8>>(64);
        let (session_tx, mut plain_out) = mpsc::channel::<Vec<u8>>(256);
        if plain_in.send(hello).await.is_err() {
            return;
        }

        // Phone → session: open each frame; one that doesn't open ends the connection. Key
        // rotation is answered here (Core holds the keys), everything else goes to the session.
        let to_phone = session_tx.clone();
        let opener = {
            let channel = channel.clone();
            let (id, psk) = (&id, &psk);
            async move {
                while let Some(frame) = frames.recv().await {
                    let plain = match channel.lock().unwrap().open(&frame) {
                        Ok(p) => p,
                        Err(_) => break,
                    };
                    if let Some(answer) = self.rotation(id, psk, &plain) {
                        if let Some(a) = answer
                            && to_phone.send(a).await.is_err()
                        {
                            break;
                        }
                        continue;
                    }
                    if plain_in.send(plain).await.is_err() {
                        break;
                    }
                }
            }
        };
        // Session → phone: seal each message.
        let sealer = async {
            while let Some(plain) = plain_out.recv().await {
                let sealed = match seal(&channel, &plain) {
                    Some(s) => s,
                    None => continue,
                };
                if out.send(sealed).await.is_err() {
                    break;
                }
            }
        };
        // Ends when this phone is removed or its key changes. Any wake-up (even a missed one) re-checks the key.
        let kicked = async {
            loop {
                match kicks.recv().await {
                    Err(broadcast::error::RecvError::Closed) => break,
                    _ if !self.key_is(&id, &psk.lock().unwrap()) => break,
                    _ => {}
                }
            }
        };
        let served = tokio::select! {
            _ = opener => false,
            _ = sealer => false,
            _ = session::serve(self.hub.clone(), self.host.clone(), session_rx, session_tx) => true,
            _ = kicked => false,
        };
        // The session's last words (why it ended) are still queued: they go out before the hang-up.
        if served {
            while let Ok(plain) = plain_out.try_recv() {
                let Some(sealed) = seal(&channel, &plain) else { continue };
                if !matches!(tokio::time::timeout(LAST_WORDS, out.send(sealed)).await, Ok(Ok(()))) {
                    break;
                }
            }
        }
    }
}

impl Core {
    /// `key.new` and `key.ok` (PROTOCOL.md, "Key rotation"), with the answer to send if any; None
    /// for any other message.
    fn rotation(&self, id: &str, psk: &Mutex<[u8; 32]>, plain: &[u8]) -> Option<Option<Vec<u8>>> {
        // Cheap test first: most messages are typing.
        const KEY: &[u8] = b"\"key.";
        if !plain.windows(KEY.len()).any(|w| w == KEY) {
            return None;
        }
        let m: Value = serde_json::from_slice(plain).ok()?;
        match m.get("t").and_then(Value::as_str)? {
            "key.new" => {
                let k = {
                    let mut offered = self.offered.lock().unwrap();
                    match offered.get(id) {
                        Some(k) => *k,
                        None => {
                            let mut k = [0u8; 32];
                            if getrandom::fill(&mut k).is_err() {
                                return Some(serde_json::to_vec(&json!({ "t": "error", "req": m.get("req"), "message": "No key" })).ok());
                            }
                            offered.insert(id.to_string(), k);
                            k
                        }
                    }
                };
                Some(serde_json::to_vec(&json!({ "t": "key.new", "req": m.get("req"), "k": URL_SAFE_NO_PAD.encode(k) })).ok())
            }
            "key.ok" => {
                let k = self.offered.lock().unwrap().get(id).copied();
                if let Some(k) = k {
                    // This connection moves to the new key first, so only the others on the old one end.
                    *psk.lock().unwrap() = k;
                    self.adopt(id, k);
                }
                Some(None)
            }
            _ => None,
        }
    }

    /// The phone has `k` now: it becomes its key (kept by the host), the old one stops working,
    /// and connections still on it end.
    fn adopt(&self, id: &str, k: [u8; 32]) {
        self.offered.lock().unwrap().remove(id);
        if !self.keys.lock().unwrap().contains_key(id) {
            // Removed meanwhile: stays removed.
            return;
        }
        // The host keeps it first, and only then is it the key here: the host hands its keys back
        // at any moment (`set_keys`), and one read in between must not be the old key written
        // over the new, which would shut this phone out until the next start.
        self.host.key_changed(id, k);
        {
            let mut keys = self.keys.lock().unwrap();
            let Some(cur) = keys.get_mut(id) else { return };
            *cur = k;
        }
        let _ = self.kicks.send(id.to_string());
    }
}

fn seal(channel: &Mutex<Channel>, plain: &[u8]) -> Option<Vec<u8>> {
    channel.lock().unwrap().seal(plain).ok()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::remote::hub::{ProjectMeta, TerminalMeta};
    use serde_json::{Value, json};

    /// `on_key`: called when a phone's new key is to be kept, as the real host's config is.
    #[derive(Default)]
    pub struct Fake {
        on_key: Mutex<Option<OnKey>>,
    }
    type OnKey = Box<dyn Fn([u8; 32]) + Send + Sync>;
    impl Host for Fake {
        fn key_changed(&self, _: &str, psk: [u8; 32]) {
            if let Some(f) = &*self.on_key.lock().unwrap() {
                f(psk);
            }
        }
        fn hello(&self) -> Value {
            json!({ "name": "Mac", "os": "macos", "app": "test" })
        }
        fn write(&self, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn git_status(&self, _: &str) -> Result<Value, String> {
            Ok(json!({}))
        }
        fn git_diff(&self, _: &str, _: &str) -> Result<String, String> {
            Ok(String::new())
        }
        fn git_act(&self, _: &str, _: session::GitAct) -> Result<Option<String>, String> {
            Err("Not here".into())
        }
        fn git_staged_hunks(&self, _: &str, _: &str) -> Result<Vec<bool>, String> {
            Ok(vec![])
        }
        fn term_new(&self, _: &str, _: Option<&str>) -> Result<tokio::sync::oneshot::Receiver<Result<String, String>>, String> {
            Err("Not here".into())
        }
        fn term_kill(&self, _: &str, _: &str) -> Result<tokio::sync::oneshot::Receiver<Result<String, String>>, String> {
            Err("Not here".into())
        }
    }

    pub const PHONE: &str = "6f0b0c2e-8f43-4a0e-9b7c-0d6a1f3a2b11";
    pub const PSK: [u8; 32] = [9u8; 32];

    pub fn core() -> Arc<Core> {
        core_with(Arc::new(Fake::default()))
    }

    fn core_with(host: Arc<Fake>) -> Arc<Core> {
        let hub = Arc::new(Hub::default());
        hub.started("t1", 80, 24);
        hub.set_projects(vec![ProjectMeta {
            root: "/app".into(),
            name: "app".into(),
            attention: false,
            terminals: vec![TerminalMeta { id: "t1".into(), name: "zsh".into(), agent: None }],
        }]);
        let core = Arc::new(Core::new(hub, host, || {}));
        core.set_keys(HashMap::from([(PHONE.to_string(), PSK)]));
        core
    }

    /// The phone's first frame and its handshake state.
    pub fn first_frame(phone: &str, psk: &[u8; 32]) -> (Vec<u8>, snow::HandshakeState) {
        let (msg, hs) = noise::initiate(psk).unwrap();
        let mut f = uuid::Uuid::parse_str(phone).unwrap().as_bytes().to_vec();
        f.extend(msg);
        (f, hs)
    }

    #[tokio::test]
    async fn a_phone_connects_and_talks() {
        let core = core();
        let (to, frames) = mpsc::channel(8);
        let (out, mut from) = mpsc::channel(8);
        let c = core.clone();
        let task = tokio::spawn(async move { c.phone(frames, out).await });
        let (f, hs) = first_frame(PHONE, &PSK);
        to.send(f).await.unwrap();
        let mut ch = noise::finish(hs, &from.recv().await.unwrap()).unwrap();
        // Connected only once its first sealed message has arrived.
        assert!(!core.is_connected(PHONE));
        to.send(ch.seal(br#"{"t":"hello","v":1}"#).unwrap()).await.unwrap();
        let hello: Value = serde_json::from_slice(&ch.open(&from.recv().await.unwrap()).unwrap()).unwrap();
        assert_eq!(hello["name"], "Mac");
        assert!(core.is_connected(PHONE));
        // Removing the phone ends its connection.
        core.set_keys(HashMap::new());
        task.await.unwrap();
        assert!(!core.is_connected(PHONE));
    }

    #[tokio::test]
    async fn a_new_key_for_the_same_phone_ends_its_old_connection() {
        let core = core();
        let (to, frames) = mpsc::channel(8);
        let (out, mut from) = mpsc::channel(8);
        let c = core.clone();
        let task = tokio::spawn(async move { c.phone(frames, out).await });
        let (f, hs) = first_frame(PHONE, &PSK);
        to.send(f).await.unwrap();
        let mut ch = noise::finish(hs, &from.recv().await.unwrap()).unwrap();
        to.send(ch.seal(br#"{"t":"hello","v":1}"#).unwrap()).await.unwrap();
        from.recv().await.unwrap();
        // Same id, paired again with a new key: the connection on the old key ends.
        core.set_keys(HashMap::from([(PHONE.to_string(), [7u8; 32])]));
        tokio::time::timeout(std::time::Duration::from_secs(2), task).await.unwrap().unwrap();
        assert!(!core.is_connected(PHONE));
    }

    /// Connects with `psk` and says hello: the phone's channel, or None when not accepted.
    async fn connect(core: &Arc<Core>, psk: &[u8; 32]) -> Option<(mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>, Channel)> {
        let (to, frames) = mpsc::channel(8);
        let (out, mut from) = mpsc::channel(8);
        let c = core.clone();
        tokio::spawn(async move { c.phone(frames, out).await });
        let (f, hs) = first_frame(PHONE, psk);
        to.send(f).await.unwrap();
        let mut ch = noise::finish(hs, &from.recv().await?).unwrap();
        to.send(ch.seal(br#"{"t":"hello","v":1}"#).unwrap()).await.unwrap();
        // Opened, so the next message's nonce follows.
        ch.open(&from.recv().await?).unwrap();
        Some((to, from, ch))
    }

    async fn say(to: &mpsc::Sender<Vec<u8>>, ch: &mut Channel, m: Value) {
        to.send(ch.seal(m.to_string().as_bytes()).unwrap()).await.unwrap();
    }

    async fn hear(from: &mut mpsc::Receiver<Vec<u8>>, ch: &mut Channel) -> Option<Value> {
        let f = tokio::time::timeout(std::time::Duration::from_secs(2), from.recv()).await.ok()??;
        serde_json::from_slice(&ch.open(&f).unwrap()).ok()
    }

    /// The `key.new` answer's key.
    async fn new_key(to: &mpsc::Sender<Vec<u8>>, from: &mut mpsc::Receiver<Vec<u8>>, ch: &mut Channel) -> [u8; 32] {
        say(to, ch, json!({ "t": "key.new", "req": 7 })).await;
        loop {
            let m = hear(from, ch).await.unwrap();
            if m["t"] == "key.new" {
                assert_eq!(m["req"], 7);
                return URL_SAFE_NO_PAD.decode(m["k"].as_str().unwrap()).unwrap().try_into().unwrap();
            }
        }
    }

    #[tokio::test]
    async fn a_phone_takes_a_new_key_and_the_qr_key_stops_working() {
        let core = core();
        let (to, mut from, mut ch) = connect(&core, &PSK).await.unwrap();
        let k = new_key(&to, &mut from, &mut ch).await;
        assert_ne!(k, PSK);
        say(&to, &mut ch, json!({ "t": "key.ok" })).await;
        // This connection goes on.
        say(&to, &mut ch, json!({ "t": "ping" })).await;
        loop {
            if hear(&mut from, &mut ch).await.unwrap()["t"] == "pong" {
                break;
            }
        }
        // The key from the QR code is no good any more; the new one is.
        assert!(connect(&core, &PSK).await.is_none());
        assert!(connect(&core, &k).await.is_some());
    }

    #[tokio::test]
    async fn the_hosts_keys_handed_back_during_a_rotation_dont_undo_it() {
        let host = Arc::new(Fake::default());
        let core = core_with(host.clone());
        // The host's config, and its keys handed to Core from it (as after a pairing or a
        // forgotten phone), right before and right after the new key is written there.
        let config = Arc::new(Mutex::new(PSK));
        let (weak, kept) = (Arc::downgrade(&core), config.clone());
        *host.on_key.lock().unwrap() = Some(Box::new(move |k| {
            let core = weak.upgrade().unwrap();
            let sync = || core.set_keys(HashMap::from([(PHONE.to_string(), *kept.lock().unwrap())]));
            sync();
            *kept.lock().unwrap() = k;
            sync();
        }));
        let (to, mut from, mut ch) = connect(&core, &PSK).await.unwrap();
        let k = new_key(&to, &mut from, &mut ch).await;
        say(&to, &mut ch, json!({ "t": "key.ok" })).await;
        // The connection that took the key goes on, and the key is the new one everywhere.
        say(&to, &mut ch, json!({ "t": "ping" })).await;
        loop {
            if hear(&mut from, &mut ch).await.unwrap()["t"] == "pong" {
                break;
            }
        }
        assert_eq!(*config.lock().unwrap(), k);
        assert!(connect(&core, &PSK).await.is_none());
        assert!(connect(&core, &k).await.is_some());
    }

    #[tokio::test]
    async fn a_lost_confirmation_still_lets_the_new_key_in() {
        let core = core();
        let (to, mut from, mut ch) = connect(&core, &PSK).await.unwrap();
        let k = new_key(&to, &mut from, &mut ch).await;
        // Asking again offers the same key.
        assert_eq!(new_key(&to, &mut from, &mut ch).await, k);
        drop(to);
        // No key.ok arrived: the old key works, and so does the offered one, which then becomes the key.
        assert!(connect(&core, &k).await.is_some());
        assert!(connect(&core, &PSK).await.is_none());
    }

    #[tokio::test]
    async fn unknown_phones_and_wrong_keys_get_nothing() {
        let core = core();
        for (phone, psk) in [("00000000-0000-4000-8000-000000000000", PSK), (PHONE, [1u8; 32])] {
            let (to, frames) = mpsc::channel(8);
            let (out, mut from) = mpsc::channel(8);
            to.send(first_frame(phone, &psk).0).await.unwrap();
            core.phone(frames, out).await;
            assert!(from.recv().await.is_none());
        }
    }
}
