//! Remote from anywhere (Pro): one outbound WebSocket to the KlikVibe relay,
//! carrying every phone that connects through it. Each phone is a channel
//! (PROTOCOL.md, "Relay framing") served by Core exactly like a LAN
//! connection, so the relay only ever sees Noise ciphertext.

use super::core::Core;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

/// The phones on the connection: channel → where its frames go.
type Channels = Arc<Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>>;

/// Why a connection ended.
#[derive(Debug, PartialEq)]
pub enum Ended {
    /// It ran and closed (the server restarted, the network dropped): reconnect soon.
    Closed,
    /// The server refused: not Pro (403) or not linked (401). Wait for the heartbeat to say otherwise.
    Refused,
    /// Couldn't connect at all (offline).
    Unreachable,
}

/// The relay's WebSocket address for this server (`https://…` → `wss://…/v1/relay/desktop`).
pub fn url(server: &str) -> String {
    let ws = server.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
    format!("{}/v1/relay/desktop", ws.trim_end_matches('/'))
}

/// The relay pings every 30 seconds: this long without any frame means the link is dead
/// (sleep, a network change) even if the socket never said so.
const SILENCE: Duration = Duration::from_secs(90);

/// One connection, until it ends. `up` is true while it is connected. `revoke` is called with
/// a phone removed from the account.
pub async fn connect(core: Arc<Core>, url: &str, token: &str, up: &AtomicBool, revoke: &(dyn Fn(&str) + Sync)) -> Ended {
    let mut req = match url.into_client_request() {
        Ok(r) => r,
        Err(_) => return Ended::Unreachable,
    };
    match HeaderValue::from_str(&format!("Device {token}")) {
        Ok(v) => req.headers_mut().insert("authorization", v),
        Err(_) => return Ended::Refused,
    };
    let ws = match tokio::time::timeout(Duration::from_secs(20), tokio_tungstenite::connect_async(req)).await {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(tokio_tungstenite::tungstenite::Error::Http(res))) if matches!(res.status().as_u16(), 401 | 403) => return Ended::Refused,
        _ => return Ended::Unreachable,
    };
    up.store(true, Ordering::Relaxed);
    let (mut sink, mut stream) = ws.split();
    // Everything to the relay goes through one writer.
    let (out, mut out_rx) = mpsc::channel::<Message>(256);
    let writer = tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            if sink.send(m).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });
    let channels: Channels = Arc::default();
    // The phones' tasks: aborted together when this connection ends or Remote is turned off.
    let mut tasks = tokio::task::JoinSet::new();

    while let Ok(Some(Ok(msg))) = tokio::time::timeout(SILENCE, stream.next()).await {
        match msg {
            Message::Text(t) => {
                let Ok(m) = serde_json::from_str::<Value>(&t) else { continue };
                if m.get("t").and_then(Value::as_str) == Some("revoke") {
                    if let Some(id) = m.get("phone").and_then(Value::as_str) {
                        revoke(id);
                    }
                    continue;
                }
                let Some(ch) = m.get("ch").and_then(Value::as_u64).and_then(|c| u32::try_from(c).ok()) else { continue };
                match m.get("t").and_then(Value::as_str) {
                    Some("open") => open(&core, ch, &out, &channels, &mut tasks),
                    // The phone left: dropping its sender ends its session.
                    Some("close") => {
                        channels.lock().unwrap().remove(&ch);
                    }
                    _ => {}
                }
            }
            Message::Binary(b) if b.len() >= 4 => {
                let ch = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
                let tx = channels.lock().unwrap().get(&ch).cloned();
                if let Some(tx) = tx {
                    // Waiting here slows the relay down instead of buffering without end.
                    let _ = tx.send(b[4..].to_vec()).await;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    up.store(false, Ordering::Relaxed);
    channels.lock().unwrap().clear();
    tasks.abort_all();
    drop(out);
    writer.abort();
    Ended::Closed
}

/// A phone arrived on channel `ch`: Core serves it, its frames go out tagged with `ch`.
fn open(core: &Arc<Core>, ch: u32, out: &mpsc::Sender<Message>, channels: &Channels, tasks: &mut tokio::task::JoinSet<()>) {
    let (frames_tx, frames) = mpsc::channel::<Vec<u8>>(64);
    let (phone_out, mut phone_rx) = mpsc::channel::<Vec<u8>>(64);
    channels.lock().unwrap().insert(ch, frames_tx);
    let tag = ch.to_be_bytes();
    let to_relay = out.clone();
    tasks.spawn(async move {
        while let Some(frame) = phone_rx.recv().await {
            let mut b = Vec::with_capacity(4 + frame.len());
            b.extend_from_slice(&tag);
            b.extend(frame);
            if to_relay.send(Message::Binary(b.into())).await.is_err() {
                break;
            }
        }
    });
    let (core, out, channels) = (core.clone(), out.clone(), channels.clone());
    tasks.spawn(async move {
        core.phone(frames, phone_out).await;
        // Over (hung up, wrong key, removed): tell the relay to drop the phone.
        if channels.lock().unwrap().remove(&ch).is_some() {
            let _ = out.send(Message::Text(json!({ "t": "close", "ch": ch }).to_string().into())).await;
        }
    });
}

/// Keeps the relay connected while `token()` gives a token (Remote on, linked, Pro),
/// reconnecting with backoff.
pub async fn run(
    core: Arc<Core>,
    server: String,
    token: impl Fn() -> Option<String>,
    up: Arc<AtomicBool>,
    changed: impl Fn(),
    revoke: impl Fn(&str) + Sync,
) {
    let mut backoff = Duration::from_secs(2);
    loop {
        let Some(t) = token() else {
            tokio::time::sleep(Duration::from_secs(20)).await;
            continue;
        };
        let started = std::time::Instant::now();
        let ended = connect(core.clone(), &url(&server), &t, &up, &revoke).await;
        changed();
        let wait = if ended == Ended::Refused {
            Duration::from_secs(300)
        } else if started.elapsed() > Duration::from_secs(60) {
            // A connection that lasted a while starts the backoff over.
            backoff = Duration::from_secs(2);
            backoff
        } else {
            let w = backoff;
            backoff = (backoff * 2).min(Duration::from_secs(60));
            w
        };
        tokio::time::sleep(wait).await;
    }
}

#[cfg(test)]
#[allow(clippy::result_large_err)] // tungstenite's handshake callback type
mod tests {
    use super::*;
    use crate::remote::core::tests::{PHONE, PSK, core, first_frame};
    use crate::remote::noise;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    #[test]
    fn relay_urls() {
        assert_eq!(url("https://klikvibe.ashafizullah.com"), "wss://klikvibe.ashafizullah.com/v1/relay/desktop");
        assert_eq!(url("http://localhost:3120/"), "ws://localhost:3120/v1/relay/desktop");
    }

    /// A pretend relay: checks the token, opens channel 7, and passes a phone's frames.
    #[tokio::test]
    async fn a_phone_through_the_relay() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let relay = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut auth = String::new();
            let mut ws = tokio_tungstenite::accept_hdr_async(tcp, |req: &Request, res: Response| {
                auth = req.headers().get("authorization").unwrap().to_str().unwrap().to_string();
                Ok(res)
            })
            .await
            .unwrap();
            ws.send(Message::Text(r#"{"t":"open","ch":7}"#.into())).await.unwrap();
            let (first, hs) = first_frame(PHONE, &PSK);
            let mut f = 7u32.to_be_bytes().to_vec();
            f.extend(first);
            ws.send(Message::Binary(f.into())).await.unwrap();
            let answer = ws.next().await.unwrap().unwrap().into_data();
            assert_eq!(&answer[..4], &7u32.to_be_bytes());
            let mut ch = noise::finish(hs, &answer[4..]).unwrap();
            let mut f = 7u32.to_be_bytes().to_vec();
            f.extend(ch.seal(br#"{"t":"hello","v":1}"#).unwrap());
            ws.send(Message::Binary(f.into())).await.unwrap();
            let hello = ws.next().await.unwrap().unwrap().into_data();
            let hello: Value = serde_json::from_slice(&ch.open(&hello[4..]).unwrap()).unwrap();
            assert_eq!(hello["t"], "hello");
            // The phone leaves; then the relay goes away.
            ws.send(Message::Text(r#"{"t":"close","ch":7}"#.into())).await.unwrap();
            ws.close(None).await.unwrap();
            auth
        });
        let up = AtomicBool::new(false);
        let ended = connect(core(), &format!("ws://{addr}/v1/relay/desktop"), "secret", &up, &|_| {}).await;
        assert_eq!(ended, Ended::Closed);
        assert_eq!(relay.await.unwrap(), "Device secret");
        assert!(!up.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn a_removed_phone_is_forgotten() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            ws.send(Message::Text(r#"{"t":"revoke","phone":"p-1"}"#.into())).await.unwrap();
            ws.close(None).await.unwrap();
        });
        let up = AtomicBool::new(false);
        let gone = std::sync::Mutex::new(Vec::new());
        connect(core(), &format!("ws://{addr}/x"), "t", &up, &|id| gone.lock().unwrap().push(id.to_string())).await;
        assert_eq!(gone.into_inner().unwrap(), ["p-1"]);
    }

    #[tokio::test]
    async fn a_refusal_is_told_apart() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let _ = tokio_tungstenite::accept_hdr_async(tcp, |_: &Request, _: Response| {
                let mut r = tokio_tungstenite::tungstenite::http::Response::new(Some("not pro".to_string()));
                *r.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
                Err(r)
            })
            .await;
        });
        let up = AtomicBool::new(false);
        assert_eq!(connect(core(), &format!("ws://{addr}/x"), "t", &up, &|_| {}).await, Ended::Refused);
        assert_eq!(connect(core(), "ws://127.0.0.1:1/x", "t", &up, &|_| {}).await, Ended::Unreachable);
    }
}
