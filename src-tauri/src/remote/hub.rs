//! What the phone sees: the open projects, their terminals, and each
//! terminal's recent output (so a phone that opens it later sees what came
//! before). Fed by terminal.rs (output, input, size, exit) and by the window
//! (which projects are open, and what each terminal is called and runs).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// Output kept per terminal for a phone that opens it later.
pub const SCROLLBACK: usize = 256 * 1024;
/// Output after this much silence starts a new burst of work (TerminalSession.svelte has the same).
const QUIET: Duration = Duration::from_secs(6);
/// A burst shorter than this (a command's output, typing) isn't an agent at work.
const BUSY: Duration = Duration::from_secs(3);
/// Output this soon after input is the echo of typing.
const ECHO: Duration = Duration::from_secs(1);

/// What the window reports about an open project.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectMeta {
    pub root: String,
    pub name: String,
    #[serde(default)]
    pub attention: bool,
    /// The project's terminals, in tab order.
    pub terminals: Vec<TerminalMeta>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct TerminalMeta {
    /// The session id from terminal.rs.
    pub id: String,
    pub name: String,
    /// The agent running in it (agents.ts id), if any.
    pub agent: Option<String>,
}

/// An agent the window knows (agents.ts), one the phone may start with `term.new`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AgentMeta {
    pub id: String,
    pub label: String,
}

/// A project as the phone gets it (PROTOCOL.md, `Project`).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ProjectView {
    pub root: String,
    pub name: String,
    pub attention: bool,
    pub terminals: Vec<TerminalView>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TerminalView {
    pub id: String,
    pub name: String,
    pub agent: Option<String>,
    pub status: &'static str,
}

#[derive(Clone, Debug)]
pub enum Event {
    Data {
        id: String,
        /// Where these bytes start in the terminal's whole output, so a phone that just opened
        /// the terminal can skip what its scrollback already had.
        offset: u64,
        bytes: Arc<[u8]>,
    },
    Exit {
        id: String,
        code: Option<u32>,
    },
    /// The project list (or a terminal's status) changed.
    Projects,
    Attention {
        root: String,
        term: String,
        title: String,
        body: String,
    },
    /// A terminal's size changed on the computer.
    Resize {
        id: String,
    },
}

struct Term {
    buf: VecDeque<u8>,
    /// Bytes printed since the terminal started (`buf` holds the last of them).
    total: u64,
    cols: u16,
    rows: u16,
    exited: Option<Option<u32>>,
    last_in: Option<Instant>,
    last_out: Option<Instant>,
    burst_start: Option<Instant>,
}

impl Term {
    fn status(&self, agent: bool, now: Instant) -> &'static str {
        if self.exited.is_some() {
            return "exited";
        }
        if !agent {
            return "idle";
        }
        match (self.last_out, self.burst_start) {
            (Some(out), Some(start)) if now.duration_since(out) < QUIET && out.duration_since(start) >= BUSY => "busy",
            _ => "waiting",
        }
    }
}

#[derive(Default)]
struct State {
    projects: Vec<ProjectMeta>,
    /// Reported once by the window at startup; empty until then.
    agents: Vec<AgentMeta>,
    terms: HashMap<String, Term>,
    /// The last statuses sent, to tell when one changes.
    sent: Vec<ProjectView>,
}

pub struct Hub {
    state: Mutex<State>,
    tx: broadcast::Sender<Event>,
}

impl Default for Hub {
    fn default() -> Self {
        Self { state: Mutex::default(), tx: broadcast::channel(1024).0 }
    }
}

/// A terminal as a phone opening it gets it: its size and output so far, and the events from then on.
pub struct Opened {
    pub cols: u16,
    pub rows: u16,
    pub scrollback: Vec<u8>,
    /// The output offset right after `scrollback`.
    pub end: u64,
    pub exited: Option<Option<u32>>,
}

impl Hub {
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    // ---- from terminal.rs ----

    pub fn started(&self, id: &str, cols: u16, rows: u16) {
        let term = Term { buf: VecDeque::new(), total: 0, cols, rows, exited: None, last_in: None, last_out: None, burst_start: None };
        self.state.lock().unwrap().terms.insert(id.to_string(), term);
    }

    pub fn output(&self, id: &str, bytes: &[u8]) {
        let mut st = self.state.lock().unwrap();
        let Some(t) = st.terms.get_mut(id) else { return };
        let offset = t.total;
        t.total += bytes.len() as u64;
        t.buf.extend(bytes);
        if t.buf.len() > SCROLLBACK {
            let extra = t.buf.len() - SCROLLBACK;
            t.buf.drain(..extra);
        }
        let now = Instant::now();
        if t.last_in.is_none_or(|i| now.duration_since(i) >= ECHO) {
            if t.last_out.is_none_or(|o| now.duration_since(o) > QUIET) {
                t.burst_start = Some(now);
            }
            t.last_out = Some(now);
        }
        // Sent under the lock, so a phone opening the terminal right now gets every byte once.
        let _ = self.tx.send(Event::Data { id: id.to_string(), offset, bytes: bytes.into() });
    }

    pub fn input(&self, id: &str) {
        if let Some(t) = self.state.lock().unwrap().terms.get_mut(id) {
            t.last_in = Some(Instant::now());
        }
    }

    pub fn resized(&self, id: &str, cols: u16, rows: u16) {
        if let Some(t) = self.state.lock().unwrap().terms.get_mut(id)
            && (t.cols, t.rows) != (cols, rows)
        {
            t.cols = cols;
            t.rows = rows;
            let _ = self.tx.send(Event::Resize { id: id.to_string() });
        }
    }

    pub fn exited(&self, id: &str, code: Option<u32>) {
        let mut st = self.state.lock().unwrap();
        if let Some(t) = st.terms.get_mut(id) {
            t.exited = Some(code);
            let _ = self.tx.send(Event::Exit { id: id.to_string(), code });
        }
    }

    /// A tab closed on the computer. Phones watching it are told it ended (its process is killed,
    /// and the exit that follows would find no terminal to report).
    pub fn closed(&self, id: &str) {
        let gone = self.state.lock().unwrap().terms.remove(id);
        if gone.is_some_and(|t| t.exited.is_none()) {
            let _ = self.tx.send(Event::Exit { id: id.to_string(), code: None });
        }
        let _ = self.tx.send(Event::Projects);
    }

    // ---- from the window ----

    pub fn set_projects(&self, projects: Vec<ProjectMeta>) {
        let mut st = self.state.lock().unwrap();
        if st.projects != projects {
            st.projects = projects;
            st.sent = Self::views(&st, Instant::now());
            let _ = self.tx.send(Event::Projects);
        }
    }

    pub fn set_agents(&self, agents: Vec<AgentMeta>) {
        self.state.lock().unwrap().agents = agents;
    }

    pub fn attention(&self, root: &str, term: &str, title: &str, body: &str) {
        let _ = self.tx.send(Event::Attention { root: root.into(), term: term.into(), title: title.into(), body: body.into() });
    }

    // ---- for the phone ----

    fn views(st: &State, now: Instant) -> Vec<ProjectView> {
        st.projects
            .iter()
            .map(|p| ProjectView {
                root: p.root.clone(),
                name: p.name.clone(),
                attention: p.attention,
                terminals: p
                    .terminals
                    .iter()
                    .filter_map(|t| {
                        let term = st.terms.get(&t.id)?;
                        Some(TerminalView {
                            id: t.id.clone(),
                            name: t.name.clone(),
                            agent: t.agent.clone(),
                            status: term.status(t.agent.is_some(), now),
                        })
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn projects(&self) -> Vec<ProjectView> {
        Self::views(&self.state.lock().unwrap(), Instant::now())
    }

    /// Statuses move with time (busy → waiting) without any event: called every
    /// couple of seconds, it tells the phones when one has changed.
    pub fn tick(&self) {
        let mut st = self.state.lock().unwrap();
        let now = Self::views(&st, Instant::now());
        if now != st.sent {
            st.sent = now;
            let _ = self.tx.send(Event::Projects);
        }
    }

    /// The root of the project a terminal is in, if it is one the phone may see.
    pub fn root_of(&self, id: &str) -> Option<String> {
        let st = self.state.lock().unwrap();
        st.projects.iter().find(|p| p.terminals.iter().any(|t| t.id == id)).map(|p| p.root.clone())
    }

    pub fn has_project(&self, root: &str) -> bool {
        self.state.lock().unwrap().projects.iter().any(|p| p.root == root)
    }

    pub fn agents(&self) -> Vec<AgentMeta> {
        self.state.lock().unwrap().agents.clone()
    }

    /// The agent (agents.ts id) running in a terminal, as the window last reported it.
    pub fn agent_of(&self, id: &str) -> Option<String> {
        let st = self.state.lock().unwrap();
        st.projects.iter().flat_map(|p| &p.terminals).find(|t| t.id == id).and_then(|t| t.agent.clone())
    }

    /// Starts following a terminal. `events` must have been subscribed before
    /// this call; output after it arrives there, output before it is in `scrollback`.
    pub fn open(&self, id: &str) -> Option<Opened> {
        let st = self.state.lock().unwrap();
        let t = st.terms.get(id)?;
        Some(Opened { cols: t.cols, rows: t.rows, scrollback: t.buf.iter().copied().collect(), end: t.total, exited: t.exited })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, agent: Option<&str>) -> ProjectMeta {
        ProjectMeta {
            root: "/p".into(),
            name: "p".into(),
            attention: false,
            terminals: vec![TerminalMeta { id: id.into(), name: "Claude".into(), agent: agent.map(Into::into) }],
        }
    }

    #[test]
    fn scrollback_keeps_the_newest_output() {
        let hub = Hub::default();
        hub.started("a", 80, 24);
        hub.output("a", &vec![b'x'; SCROLLBACK]);
        hub.output("a", b"tail");
        let o = hub.open("a").unwrap();
        assert_eq!(o.scrollback.len(), SCROLLBACK);
        assert!(o.scrollback.ends_with(b"tail"));
        assert_eq!((o.cols, o.rows), (80, 24));
    }

    #[test]
    fn only_terminals_the_window_reported_are_shown() {
        let hub = Hub::default();
        hub.started("a", 80, 24);
        hub.started("hidden", 80, 24);
        hub.set_projects(vec![meta("a", None), ProjectMeta { root: "/q".into(), ..meta("gone", None) }]);
        let p = hub.projects();
        assert_eq!(p[0].terminals.len(), 1);
        assert_eq!(p[0].terminals[0].status, "idle");
        assert!(p[1].terminals.is_empty());
        assert_eq!(hub.root_of("a").as_deref(), Some("/p"));
        assert_eq!(hub.root_of("hidden"), None);
    }

    #[test]
    fn the_agent_of_a_terminal() {
        let hub = Hub::default();
        hub.set_projects(vec![meta("a", Some("claude")), ProjectMeta { root: "/q".into(), ..meta("b", None) }]);
        assert_eq!(hub.agent_of("a").as_deref(), Some("claude"));
        assert_eq!(hub.agent_of("b"), None);
        assert_eq!(hub.agent_of("nope"), None);
    }

    #[test]
    fn statuses() {
        let t0 = Instant::now();
        let mut t =
            Term { buf: VecDeque::new(), total: 0, cols: 80, rows: 24, exited: None, last_in: None, last_out: None, burst_start: None };
        assert_eq!(t.status(false, t0), "idle");
        assert_eq!(t.status(true, t0), "waiting");
        t.burst_start = Some(t0);
        t.last_out = Some(t0 + Duration::from_secs(4));
        assert_eq!(t.status(true, t0 + Duration::from_secs(5)), "busy");
        assert_eq!(t.status(true, t0 + Duration::from_secs(11)), "waiting");
        t.exited = Some(Some(0));
        assert_eq!(t.status(true, t0), "exited");
    }

    #[tokio::test]
    async fn a_closed_tab_reports_its_end() {
        let hub = Hub::default();
        hub.started("a", 80, 24);
        let mut rx = hub.subscribe();
        hub.closed("a");
        assert!(matches!(rx.recv().await.unwrap(), Event::Exit { ref id, code: None } if id == "a"));
        assert!(matches!(rx.recv().await.unwrap(), Event::Projects));
        // The exit that follows the kill finds nothing to report twice.
        hub.exited("a", Some(0));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn events_reach_subscribers() {
        let hub = Hub::default();
        let mut rx = hub.subscribe();
        hub.started("a", 80, 24);
        hub.set_projects(vec![meta("a", Some("claude"))]);
        hub.output("a", b"hi");
        hub.exited("a", Some(1));
        assert!(matches!(rx.recv().await.unwrap(), Event::Projects));
        assert!(matches!(rx.recv().await.unwrap(), Event::Data { bytes, .. } if &*bytes == b"hi"));
        assert!(matches!(rx.recv().await.unwrap(), Event::Exit { code: Some(1), .. }));
        // The same list again sends nothing.
        hub.set_projects(vec![meta("a", Some("claude"))]);
        hub.attention("/p", "a", "Claude · p", "waiting");
        assert!(matches!(rx.recv().await.unwrap(), Event::Attention { .. }));
    }
}
