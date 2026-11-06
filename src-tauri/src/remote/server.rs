//! Remote on the local network: a WebSocket at `ws://<address>:<port>/remote`
//! for phones on the same Wi-Fi, announced over mDNS as `_klikvibe._tcp` so a
//! phone finds the computer again when its address changes. Every connection
//! goes through Core: a phone without a paired key gets no further than its
//! first frame.

use super::core::Core;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use futures::{SinkExt, StreamExt};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{mpsc, oneshot};

pub const DEFAULT_PORT: u16 = 47615;
const SERVICE: &str = "_klikvibe._tcp.local.";

pub struct Running {
    pub port: u16,
    stop: Option<oneshot::Sender<()>>,
    mdns: Option<mdns_sd::ServiceDaemon>,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(d) = self.mdns.take() {
            let _ = d.shutdown();
        }
    }
}

impl Running {
    /// Announces the computer under `desktop_id` (or stops announcing it), on the same port: the
    /// port stays what paired phones have in their QR code.
    pub fn reannounce(&mut self, desktop_id: Option<&str>, name: &str) {
        if let Some(d) = self.mdns.take() {
            let _ = d.shutdown();
        }
        self.mdns = desktop_id.and_then(|id| announce(id, name, self.port));
    }
}

/// Connections at once; more are turned away (each holds a socket until its handshake times out).
const MAX_CONNECTIONS: usize = 32;
static OPEN: AtomicUsize = AtomicUsize::new(0);

/// Counts a connection while it lives.
struct Slot;
impl Drop for Slot {
    fn drop(&mut self) {
        OPEN.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn upgrade(State(core): State<Arc<Core>>, headers: HeaderMap, ws: WebSocketUpgrade) -> Response {
    // The phone app sends no Origin; a web page does. A page the user visits can't reach in.
    if headers.contains_key("origin") {
        return StatusCode::FORBIDDEN.into_response();
    }
    if OPEN.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
        OPEN.fetch_sub(1, Ordering::Relaxed);
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let slot = Slot;
    ws.max_message_size(super::noise::MAX_MESSAGE + 64).on_upgrade(move |socket| async move {
        let _slot = slot;
        serve(core, socket).await
    })
}

/// Carries one WebSocket's binary frames to and from Core.
pub async fn serve(core: Arc<Core>, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (frames_tx, frames) = mpsc::channel::<Vec<u8>>(64);
    let (out, mut out_rx) = mpsc::channel::<Vec<u8>>(64);
    let reader = async move {
        while let Some(Ok(m)) = stream.next().await {
            match m {
                Message::Binary(b) => {
                    if frames_tx.send(b.to_vec()).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };
    let writer = async move {
        while let Some(b) = out_rx.recv().await {
            if sink.send(Message::Binary(b.into())).await.is_err() {
                return;
            }
        }
        let _ = sink.close().await;
    };
    // The writer finishes after Core drops `out`, so the close reaches the phone.
    let phone = core.phone(frames, out);
    tokio::join!(
        async {
            tokio::select! { _ = reader => {}, _ = phone => {} }
        },
        writer
    );
}

/// The computer's addresses a phone on the same network can reach (private IPv4).
pub fn lan_addresses() -> Vec<String> {
    let mut out: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if v4.is_private() || is_cgnat(v4) => Some(v4.to_string()),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// 100.64.0.0/10: Tailscale and some carrier networks.
fn is_cgnat(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 100 && (64..128).contains(&b)
}

/// Starts listening on all interfaces, on `port` if it is free (else any port),
/// and announces the computer as `desktop_id` (once it has one).
pub async fn start(core: Arc<Core>, port: u16, desktop_id: Option<&str>, name: &str) -> std::io::Result<Running> {
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind(("0.0.0.0", 0)).await?,
    };
    let port = listener.local_addr()?.port();
    let (stop, stopped) = oneshot::channel::<()>();
    let app = Router::new().route("/remote", get(upgrade)).with_state(core);
    tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await;
    });
    let mdns = desktop_id.and_then(|id| announce(id, name, port));
    Ok(Running { port, stop: Some(stop), mdns })
}

fn announce(desktop_id: &str, name: &str, port: u16) -> Option<mdns_sd::ServiceDaemon> {
    let daemon = mdns_sd::ServiceDaemon::new().ok()?;
    let short: String = desktop_id.chars().filter(char::is_ascii_alphanumeric).take(8).collect();
    let host = format!("klikvibe-{short}.local.");
    let props = [("d", desktop_id), ("n", name)];
    let info = mdns_sd::ServiceInfo::new(SERVICE, desktop_id, &host, "", port, &props[..]).ok()?.enable_addr_auto();
    daemon.register(info).ok()?;
    Some(daemon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::core::tests::{PHONE, PSK, core, first_frame};
    use crate::remote::noise;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    #[tokio::test]
    async fn a_phone_over_the_lan() {
        let running = start(core(), 0, None, "Mac").await.unwrap();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/remote", running.port)).await.unwrap();
        let (f, hs) = first_frame(PHONE, &PSK);
        ws.send(WsMessage::Binary(f.into())).await.unwrap();
        let answer = ws.next().await.unwrap().unwrap().into_data();
        let mut ch = noise::finish(hs, &answer).unwrap();
        ws.send(WsMessage::Binary(ch.seal(br#"{"t":"hello","v":1}"#).unwrap().into())).await.unwrap();
        let hello: serde_json::Value = serde_json::from_slice(&ch.open(&ws.next().await.unwrap().unwrap().into_data()).unwrap()).unwrap();
        assert_eq!(hello["t"], "hello");
        let projects: serde_json::Value =
            serde_json::from_slice(&ch.open(&ws.next().await.unwrap().unwrap().into_data()).unwrap()).unwrap();
        assert_eq!(projects["projects"][0]["name"], "app");
    }

    #[tokio::test]
    async fn a_stranger_is_hung_up_on() {
        let running = start(core(), 0, None, "Mac").await.unwrap();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/remote", running.port)).await.unwrap();
        ws.send(WsMessage::Binary(first_frame(PHONE, &[3u8; 32]).0.into())).await.unwrap();
        // Closed without an answer.
        loop {
            match ws.next().await {
                Some(Ok(WsMessage::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(WsMessage::Binary(_))) => panic!("answered a stranger"),
                _ => {}
            }
        }
    }

    #[test]
    fn addresses() {
        assert!(is_cgnat("100.101.1.2".parse().unwrap()));
        assert!(!is_cgnat("100.1.1.2".parse().unwrap()));
        assert!(lan_addresses().iter().all(|a| a.parse::<Ipv4Addr>().is_ok()));
    }
}
