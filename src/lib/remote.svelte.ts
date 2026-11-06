// KlikVibe Remote in the window: its status for the dialog and the title bar,
// and the open projects and their terminals, reported to Rust (remote/hub.rs)
// for the phone while Remote is on.
import { listen } from "@tauri-apps/api/event";
import { ipc, type RemoteProject, type RemoteStatus } from "./ipc";
import { workspace } from "./project.svelte";
import { attention } from "./attention.svelte";
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
    attention: attention.has(p.root),
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

let started = false;
export function initRemote() {
  if (started) return;
  started = true;
  refreshRemote();
  listen("remote-changed", () => refreshRemote());
  setInterval(report, 1500);
}
