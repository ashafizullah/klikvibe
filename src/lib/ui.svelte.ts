// Interface state shared across components: toasts, the busy indicator and
// the right-click menu.
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { t } from "./i18n/index.svelte";

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
  action?: { label: string; run: () => void };
}

/** A right-click menu entry, or "sep" for a divider. */
export type MenuItem = { label: string; run: () => void; disabled?: boolean; hint?: string } | "sep";

export const ui = $state({
  busy: null as string | null,
  toasts: [] as Toast[],
  menu: null as { x: number; y: number; items: MenuItem[] } | null,
  /** The modal on screen, if any. */
  dialog: null as "mcp" | null,
});

/** Shows a right-click menu at the pointer. */
export function openMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  ui.menu = { x: e.clientX, y: e.clientY, items };
}

export function closeMenu() {
  ui.menu = null;
}

/** Shows a file or folder in Finder / Explorer. */
export function reveal(path: string) {
  revealItemInDir(path).catch((e) => toast(e instanceof Error ? e.message : String(e), "error"));
}

export async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast(t("common.copied"), "ok", undefined, 1800);
  } catch (e) {
    toast(e instanceof Error ? e.message : String(e), "error");
  }
}

let nextToast = 1;
/** `ms` = 0 keeps the toast until the user closes it or takes its action. */
export function toast(text: string, kind: Toast["kind"] = "info", action?: Toast["action"], ms = kind === "error" ? 9000 : 4500) {
  const id = nextToast++;
  ui.toasts.push({ id, text, kind, action });
  if (ms > 0) setTimeout(() => dismissToast(id), ms);
}

export function dismissToast(id: number) {
  ui.toasts = ui.toasts.filter((t) => t.id !== id);
}

/** Runs `fn` with a busy indicator; errors become toasts. */
export async function busy<T>(label: string, fn: () => Promise<T>): Promise<T | undefined> {
  ui.busy = label;
  try {
    return await fn();
  } catch (e) {
    toast(e instanceof Error ? e.message : String(e), "error");
    return undefined;
  } finally {
    ui.busy = null;
  }
}

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);
export const mod = isMac ? "⌘" : "Ctrl+";
const isWindows = typeof navigator !== "undefined" && /Win/i.test(navigator.platform || navigator.userAgent);
/** "Reveal in Finder" and its counterparts. */
export const revealLabel = () => t(isMac ? "menu.revealMac" : isWindows ? "menu.revealWindows" : "menu.reveal");
