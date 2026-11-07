//! A phone's connection, whichever way it came (the LAN or the relay): the
//! handshake with the key of the phone it says it is, then its session with
//! every message sealed. Knows nothing about Tauri, so the tests run it whole.

use super::hub::Hub;
use super::noise::{self, Channel};
use super::session::{self, Host};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

/// How long a phone has for its handshake and first message.
const HANDSHAKE: std::time::Duration = std::time::Duration::from_secs(10);

pub struct Core {
    pub hub: Arc<Hub>,
    pub host: Arc<dyn Host>,
    /// Paired phones: id → key.
    keys: Mutex<HashMap<String, [u8; 32]>>,
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
        Self { hub, host, keys: Mutex::default(), connected: Mutex::default(), kicks: broadcast::channel(16).0, changed: Box::new(changed) }
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
        let Some(psk) = self.keys.lock().unwrap().get(&id).copied() else { return };
        let Ok((answer, channel)) = noise::respond(&psk, &first[16..]) else { return };
        if out.send(answer).await.is_err() {
            return;
        }
        let mut channel = channel;
        // Its first sealed message proves it holds the key (a replayed first frame can't make one).
        let Ok(Some(frame)) = tokio::time::timeout(HANDSHAKE, frames.recv()).await else { return };
        let Ok(hello) = channel.open(&frame) else { return };
        if !self.key_is(&id, &psk) {
            return;
        }
        *self.connected.lock().unwrap().entry(id.clone()).or_default() += 1;
        let _guard = Connected(self, id.clone());
        (self.changed)();

        let channel = Arc::new(Mutex::new(channel));
        let (plain_in, session_rx) = mpsc::channel::<Vec<u8>>(64);
        let (session_tx, mut plain_out) = mpsc::channel::<Vec<u8>>(256);
        if plain_in.send(hello).await.is_err() {
            return;
        }

        // Phone → session: open each frame; one that doesn't open ends the connection.
        let opener = {
            let channel = channel.clone();
            async move {
                while let Some(frame) = frames.recv().await {
                    let plain = match channel.lock().unwrap().open(&frame) {
                        Ok(p) => p,
                        Err(_) => break,
                    };
                    if plain_in.send(plain).await.is_err() {
                        break;
                    }
                }
            }
        };
        // Session → phone: seal each message.
        let sealer = async move {
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
                    _ if !self.key_is(&id, &psk) => break,
                    _ => {}
                }
            }
        };
        tokio::select! {
            _ = opener => {}
            _ = sealer => {}
            _ = session::serve(self.hub.clone(), self.host.clone(), session_rx, session_tx) => {}
            _ = kicked => {}
        }
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

    pub struct Fake;
    impl Host for Fake {
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
        let hub = Arc::new(Hub::default());
        hub.started("t1", 80, 24);
        hub.set_projects(vec![ProjectMeta {
            root: "/app".into(),
            name: "app".into(),
            attention: false,
            terminals: vec![TerminalMeta { id: "t1".into(), name: "zsh".into(), agent: None }],
        }]);
        let core = Arc::new(Core::new(hub, Arc::new(Fake), || {}));
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
