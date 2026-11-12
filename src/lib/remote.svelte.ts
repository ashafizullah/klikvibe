// KlikVibe Remote in the window: its status for the dialog and the title bar,
// and the open projects and their terminals, reported to Rust (remote/hub.rs)
// for the phone while Remote is on.
import { listen } from "@tauri-apps/api/event";
import { errorText, ipc, type RemoteProject, type RemoteStatus } from "./ipc";
import { AGENTS } from "./agents";
import { promptList } from "./prompts.svelte";
import { workspace } from "./project.svelte";
import { phoneAttention } from "./attention.svelte";
import { ui } from "./ui.svelte";

export const remote = $state({ status: null as RemoteStatus | null, busy: false, error: null as string | null });

export async function refreshRemote() {
  remote.status = await ipc.remoteStatus().catch(() => remote.status);
  // A phone scanned the code and waits for a yes: the dialog asks.
  if (remote.status?.confirm) ui.dialog = "remote";
}

/** Runs a Remote action, keeping its result (or its error) for the dialog. */
export async function remoteAction(f: () => Promise<RemoteStatus>) {
  remote.busy = true;
  remote.error = null;
  try {
    remote.status = await f();
  } catch (e) {
    remote.error = String(e);
  } finally {
    remote.busy = false;
  }
}

function projects(): RemoteProject[] {
  return workspace.projects.map((p) => ({
    root: p.root,
    name: p.current.name,
    attention: phoneAttention.has(p.root),
    terminals: p.remoteTerminals?.() ?? [],
  }));
}

let last = "";
/** Sends the projects when they changed (a tab opened, a terminal renamed or started an agent). */
function report() {
  if (!remote.status?.enabled) {
    last = "";
    return;
  }
  const list = projects();
  const json = JSON.stringify(list);
  if (json === last) return;
  last = json;
  ipc.remoteProjects(list).catch(() => (last = ""));
}

/**
 * A phone asked for a new terminal (`term.new`; Rust checked the project is open and the
 * agent is one of AGENTS). Rust hears of the terminal before it gives the phone its id,
 * so the phone can open it right away.
 */
async function termNew({ root, agent, nonce }: { root: string; agent: string | null; nonce: string }) {
  let id: string | null = null;
  let error: string | null = null;
  try {
    const p = workspace.projects.find((x) => x.root === root);
    // Every open project's panels stay mounted, so this is only a project closed just now.
    if (!p?.newTerminal) throw new Error("Project not open");
    const def = agent === null ? null : AGENTS.find((a) => a.id === agent);
    if (def === undefined) throw new Error("Unknown agent");
    id = await p.newTerminal(def);
    const list = projects();
    last = JSON.stringify(list);
    await ipc.remoteProjects(list).catch((e) => {
      last = "";
      throw e;
    });
  } catch (e) {
    id = null;
    error = errorText(e);
  }
  ipc.remoteTermCreated(nonce, id, error).catch(() => {});
}

/** A phone closed a terminal (`term.kill`; Rust checked it belongs to an open project). */
function termKill({ root, id, nonce }: { root: string; id: string; nonce: string }) {
  let error: string | null = null;
  const p = workspace.projects.find((x) => x.root === root);
  if (!p?.closeTerminal?.(id)) error = "No such terminal";
  else {
    // The phone's list drops it right away.
    const list = projects();
    last = JSON.stringify(list);
    ipc.remoteProjects(list).catch(() => (last = ""));
  }
  ipc.remoteTermCreated(nonce, error ? null : id, error).catch(() => {});
}

/** A phone opens a terminal: it gets the history as this window shows it (Rust falls back to raw output). */
async function termSnapshot({ id, lines, nonce }: { id: string; lines: number; nonce: string }) {
  let snap: { data: string; offset: number } | null = null;
  try {
    for (const p of workspace.projects) {
      const pending = p.terminalSnapshot?.(id, lines);
      if (pending) {
        snap = await pending;
        break;
      }
    }
  } catch {}
  ipc.remoteTermSnapshot(nonce, snap?.data ?? null, snap?.offset ?? 0).catch(() => {});
}

let started = false;
export function initRemote() {
  if (started) return;
  started = true;
  refreshRemote();
  // The agents a phone may start in a new terminal (by id: never a command from the phone).
  ipc.remoteAgents(AGENTS.map(({ id, label }) => ({ id, label }))).catch(() => {});
  // The Prompts menu, for the phone's own (again whenever it is edited, or the language changes).
  $effect.root(() => {
    $effect(() => {
      ipc.remotePrompts(promptList().filter((p) => p.trim())).catch(() => {});
    });
  });
  listen("remote-changed", () => refreshRemote());
  listen<{ root: string; agent: string | null; nonce: string }>("remote-term-new", (e) => termNew(e.payload));
  listen<{ root: string; id: string; nonce: string }>("remote-term-kill", (e) => termKill(e.payload));
  listen<{ id: string; lines: number; nonce: string }>("remote-term-snapshot", (e) => termSnapshot(e.payload));
  setInterval(report, 1500);
}
