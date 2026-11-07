//! Remote on the local network: a WebSocket at `ws://<address>:<port>/remote`
//! for phones on the same Wi-Fi, announced over mDNS as `_klikvibe._tcp` so a
//! phone finds the computer again when its address changes. Every connection
//! goes through Core: a phone without a paired key gets no further than its
//! first frame.

use super::core::Core;
use axum::Extension;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use futures::{SinkExt, StreamExt};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tower_service::Service;

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

/// WebSocket connections at once; more are turned away (each holds a socket until its handshake times out).
const MAX_CONNECTIONS: usize = 32;
/// TCP connections at once, before or after their upgrade, in all and from one address. Each holds
/// a file handle: without a limit, idle connections from one machine on the Wi-Fi could use up the
/// app's handles (terminals, files and git would then fail).
const MAX_SOCKETS: usize = 64;
const MAX_PER_PEER: usize = 8;
/// A connection must have sent its request this soon after connecting.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(if cfg!(test) { 1 } else { 10 });

/// Who may connect: the LAN, Tailscale, and this computer. The listener is on every interface so
/// it follows the computer from one network to the next; a public address is hung up on.
fn allowed_peer(ip: IpAddr) -> bool {
    let v4 = match ip {
        IpAddr::V4(v4) => v4,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => v4,
            None => return v6.is_loopback(),
        },
    };
    v4.is_private() || v4.is_loopback() || v4.is_link_local() || is_cgnat(v4)
}

/// Open TCP connections, in all and per address.
#[derive(Default)]
struct Peers(Mutex<(usize, HashMap<IpAddr, usize>)>);

/// One counted connection, released when its last holder (the connection, then its WebSocket) drops.
struct PeerSlot(Arc<Peers>, IpAddr);

impl Peers {
    fn take(self: &Arc<Self>, ip: IpAddr) -> Option<PeerSlot> {
        let mut g = self.0.lock().unwrap();
        let n = g.1.get(&ip).copied().unwrap_or(0);
        if g.0 >= MAX_SOCKETS || n >= MAX_PER_PEER {
            return None;
        }
        g.0 += 1;
        g.1.insert(ip, n + 1);
        Some(PeerSlot(self.clone(), ip))
    }
}

impl Drop for PeerSlot {
    fn drop(&mut self) {
        let mut g = self.0.0.lock().unwrap();
        g.0 -= 1;
        if let Some(n) = g.1.get_mut(&self.1) {
            *n -= 1;
            if *n == 0 {
                g.1.remove(&self.1);
            }
        }
    }
}
static OPEN: AtomicUsize = AtomicUsize::new(0);

/// Counts a connection while it lives.
struct Slot;
impl Drop for Slot {
    fn drop(&mut self) {
        OPEN.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn upgrade(
    State(core): State<Arc<Core>>,
    peer: Option<Extension<Arc<PeerSlot>>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
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
        // The address's count holds while the WebSocket lives.
        let _slots = (slot, peer);
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
    tauri::async_runtime::spawn(accept(listener, app, stopped));
    let mdns = desktop_id.and_then(|id| announce(id, name, port));
    Ok(Running { port, stop: Some(stop), mdns })
}

/// Takes connections until `stopped`: only from allowed addresses, within the limits, each with
/// its request read in time. Until its upgrade a connection lives at most twice the request timeout
/// (keep-alive can't hold it open); a WebSocket then lives on its own.
async fn accept(listener: tokio::net::TcpListener, app: Router, mut stopped: oneshot::Receiver<()>) {
    let peers = Arc::new(Peers::default());
    let mut conns = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut stopped => break,
            Some(_) = conns.join_next(), if !conns.is_empty() => {}
            r = listener.accept() => {
                let (stream, addr) = match r {
                    Ok(c) => c,
                    // Out of handles, say: wait instead of spinning.
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                // Dropping the stream hangs up.
                if !allowed_peer(addr.ip()) {
                    continue;
                }
                let Some(slot) = peers.take(addr.ip()) else { continue };
                let slot = Arc::new(slot);
                let app = app.clone();
                conns.spawn(async move {
                    let svc = hyper::service::service_fn(move |mut req: hyper::Request<hyper::body::Incoming>| {
                        req.extensions_mut().insert(slot.clone());
                        app.clone().call(req)
                    });
                    let conn = hyper::server::conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .header_read_timeout(REQUEST_TIMEOUT)
                        .serve_connection(TokioIo::new(stream), svc)
                        .with_upgrades();
                    // Also closes one that sends nothing at all, or keeps an idle keep-alive.
                    let _ = tokio::time::timeout(REQUEST_TIMEOUT * 2, conn).await;
                });
            }
        }
    }
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
    fn only_local_peers() {
        for ok in ["192.168.1.7", "10.0.0.2", "172.16.5.5", "100.101.1.2", "127.0.0.1", "169.254.3.4"] {
            assert!(allowed_peer(ok.parse().unwrap()), "{ok}");
        }
        for no in ["8.8.8.8", "100.1.1.2", "2606:4700::1"] {
            assert!(!allowed_peer(no.parse().unwrap()), "{no}");
        }
        assert!(allowed_peer("::ffff:192.168.1.7".parse().unwrap()));
    }

    /// True when the server hung up on `s` (read gives end of stream or an error).
    async fn hung_up(s: &mut tokio::net::TcpStream, within: Duration) -> bool {
        use tokio::io::AsyncReadExt;
        let mut b = [0u8; 1];
        matches!(tokio::time::timeout(within, s.read(&mut b)).await, Ok(Ok(0)) | Ok(Err(_)))
    }

    #[tokio::test]
    async fn one_address_gets_a_few_connections_and_idle_ones_end() {
        let running = start(core(), 0, None, "Mac").await.unwrap();
        let addr = format!("127.0.0.1:{}", running.port);
        let mut idle = Vec::new();
        for _ in 0..MAX_PER_PEER {
            idle.push(tokio::net::TcpStream::connect(&addr).await.unwrap());
        }
        // One more from the same address is hung up on at once (well before the request timeout).
        let mut extra = tokio::net::TcpStream::connect(&addr).await.unwrap();
        assert!(hung_up(&mut extra, Duration::from_millis(300)).await);
        assert!(!hung_up(&mut idle[0], Duration::from_millis(100)).await);
        // The idle ones are closed after the request timeout, which frees the address again.
        for s in &mut idle {
            assert!(hung_up(s, Duration::from_secs(5)).await);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/remote")).await.unwrap();
        ws.send(WsMessage::Binary(first_frame(PHONE, &PSK).0.into())).await.unwrap();
        assert!(matches!(ws.next().await, Some(Ok(WsMessage::Binary(_)))));
    }

    #[test]
    fn addresses() {
        assert!(is_cgnat("100.101.1.2".parse().unwrap()));
        assert!(!is_cgnat("100.1.1.2".parse().unwrap()));
        assert!(lan_addresses().iter().all(|a| a.parse::<Ipv4Addr>().is_ok()));
    }
}
