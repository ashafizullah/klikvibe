// "An agent needs you": a terminal rang its bell, an agent sent a
// notification (OSC 9 / OSC 777), or an agent that was busy has gone quiet,
// most likely waiting for an answer or done. While you are looking at that
// terminal nothing happens; otherwise its project tab gets a dot, the Dock
// icon bounces once, and macOS shows a notification.
import { getCurrentWindow, UserAttentionType } from "@tauri-apps/api/window";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { SvelteSet } from "svelte/reactivity";
import { settings } from "./settings.svelte";

/** Project roots with a terminal waiting for the user. */
export const attention = new SvelteSet<string>();

/** Not more than one notification per terminal in this long. */
const QUIET_BETWEEN = 30_000;
const last = new Map<string, number>();

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
 */
export async function needsYou(key: string, root: string, title: string, body: string, seen: boolean) {
  if (!settings.notifyAgents || seen) return;
  const now = Date.now();
  if (now - (last.get(key) ?? 0) < QUIET_BETWEEN) return;
  last.set(key, now);
  attention.add(root);
  getCurrentWindow()
    .requestUserAttention(UserAttentionType.Informational)
    .catch(() => {});
  if (await allowed()) sendNotification({ title, body });
}

/** The user looked: the project's dot goes away. */
export function seen(root: string) {
  attention.delete(root);
}
