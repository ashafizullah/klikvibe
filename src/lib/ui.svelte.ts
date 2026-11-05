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

/** An in-app message with buttons (AlertDialog). */
export interface AlertOptions {
  tone: "update" | "success" | "error" | "warning" | "danger";
  title: string;
  message: string;
  /** Release notes, rendered as safe Markdown. */
  markdown?: string;
  /** The last one is the main action. With a `danger` action, Cancel (not it) has the focus. */
  actions?: { label: string; value: string; primary?: boolean; danger?: boolean }[];
}

export interface ConfirmOptions {
  title: string;
  message: string;
  /** The label of the button that goes ahead, e.g. "Discard changes". */
  confirm: string;
  /** Something is lost for good: a red button, and Cancel has the focus. */
  danger?: boolean;
}

/** A right-click menu entry, or "sep" for a divider. */
export type MenuItem = { label: string; run: () => void; disabled?: boolean; hint?: string } | "sep";

export const ui = $state({
  busy: null as string | null,
  toasts: [] as Toast[],
  menu: null as { x: number; y: number; items: MenuItem[] } | null,
  /** The modal on screen, if any. */
  dialog: null as "mcp" | "about" | null,
  alert: null as (AlertOptions & { resolve: (value: string | null) => void }) | null,
});

/** Shows an AlertDialog; resolves with the chosen action's value, or null when dismissed. */
export function showAlert(options: AlertOptions): Promise<string | null> {
  ui.alert?.resolve(null);
  return new Promise((resolve) => (ui.alert = { ...options, resolve }));
}

/** A yes/no question in the app's own dialog; true when the user goes ahead. */
export async function confirm(o: ConfirmOptions): Promise<boolean> {
  const choice = await showAlert({
    tone: o.danger ? "danger" : "warning",
    title: o.title,
    message: o.message,
    actions: [
      { label: t("common.cancel"), value: "cancel" },
      { label: o.confirm, value: "ok", primary: !o.danger, danger: o.danger },
    ],
  });
  return choice === "ok";
}

export function closeAlert(value: string | null) {
  const alert = ui.alert;
  ui.alert = null;
  alert?.resolve(value);
}

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
