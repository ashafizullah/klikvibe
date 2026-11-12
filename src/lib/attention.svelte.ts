// "An agent needs you": a terminal rang its bell, an agent sent a
// notification (OSC 9 / OSC 777), or an agent that was busy has gone quiet,
// most likely waiting for an answer or done. While you are looking at that
// terminal nothing happens; otherwise its project tab and its terminal tab get a dot, the Dock
// icon bounces once, and macOS shows a notification.
import { getCurrentWindow, UserAttentionType } from "@tauri-apps/api/window";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { SvelteSet } from "svelte/reactivity";
import { settings } from "./settings.svelte";
import { ipc } from "./ipc";

/** Project roots with a terminal waiting for the user (the project tab's dot). */
export const attention = new SvelteSet<string>();
/** The same for the phone (KlikVibe Remote), which is told even with desktop notifications off. */
export const phoneAttention = new SvelteSet<string>();

/** What happened, for the phone's push: an agent went quiet, an agent's own notification, or the bell. */
export type AttentionKind = "waiting" | "message" | "bell";

/** Not more than one notification per terminal in this long. */
const QUIET_BETWEEN = 30_000;
const last = new Map<string, number>();

/**
 * The terminal the last notification was about, while KlikVibe was in the background: clicking
 * the notification brings the window to the front, and that terminal comes with it. Not later
 * than this, as the user may come back for something else by then.
 */
let notified: { at: number; open: () => void } | null = null;
const OPEN_WITHIN = 5 * 60_000;

/** The window came to the front: the terminal of a recent notification is shown. */
export function openNotified() {
  const n = notified;
  notified = null;
  if (n && Date.now() - n.at < OPEN_WITHIN) n.open();
}

let permission: Promise<boolean> | null = null;
function allowed(): Promise<boolean> {
  permission ??= isPermissionGranted()
    .then((ok) => ok || requestPermission().then((p) => p === "granted"))
    .catch(() => false);
  return permission;
}

/**
 * A terminal (`key`, unique per shell) in project `root` needs the user.
 * `seen`: the user is looking at it right now, so nothing is shown.
 * `session`: its session id, so a phone on KlikVibe Remote can open it.
 * `open` brings that terminal in front, for when the user comes back from the notification.
 * True when the user is told here (so the terminal's tab gets its dot too).
 */
export async function needsYou(
  key: string,
  root: string,
  title: string,
  body: string,
  seen: boolean,
  session: string | null = null,
  kind: AttentionKind = "message",
  open?: () => void,
): Promise<boolean> {
  if (seen) return false;
  const now = Date.now();
  if (now - (last.get(key) ?? 0) < QUIET_BETWEEN) return false;
  last.set(key, now);
  // The phone too (and a push with Pro), even with desktop notifications off; nothing happens while Remote is off.
  if (session) ipc.remoteAttention(root, session, title, body, kind).catch(() => {});
  phoneAttention.add(root);
  if (!settings.notifyAgents) return false;
  attention.add(root);
  getCurrentWindow()
    .requestUserAttention(UserAttentionType.Informational)
    .catch(() => {});
  if (open && !document.hasFocus()) notified = { at: now, open };
  allowed().then((ok) => ok && sendNotification({ title, body }));
  return true;
}

/** The user looked: the project's dot goes away. */
export function seen(root: string) {
  attention.delete(root);
  phoneAttention.delete(root);
}
