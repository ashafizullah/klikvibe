// Interface state shared across components: toasts, dialogs, the right-click
// menu, and small helpers (copy, reveal).
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { t } from "./i18n/index.svelte";
import { errorText } from "./ipc";

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
  /** A text field (prompt): the main action resolves with what was typed. */
  input?: { placeholder?: string; value?: string };
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
export type MenuItem = { label: string; run: () => void; disabled?: boolean; hint?: string; icon?: string; color?: string } | "sep";

export const ui = $state({
  toasts: [] as Toast[],
  menu: null as { x: number; y: number; items: MenuItem[] } | null,
  /** The modal on screen, if any. */
  dialog: null as "mcp" | "about" | "prompts" | "remote" | null,
  alert: null as (AlertOptions & { resolve: (value: string | null) => void }) | null,
});

/** Shows an AlertDialog; resolves with the chosen action's value, or null when dismissed. */
/** Alerts waiting for the one on screen to be answered. */
const queue: (AlertOptions & { resolve: (value: string | null) => void })[] = [];

export function showAlert(options: AlertOptions): Promise<string | null> {
  return new Promise((resolve) => {
    const alert = { ...options, resolve };
    // Never answer a question on screen for the user: a later alert waits its turn.
    if (ui.alert) queue.push(alert);
    else ui.alert = alert;
  });
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

/** Asks for a line of text in the app's own dialog; null when cancelled or left empty. */
export async function prompt(o: { title: string; message: string; confirm: string; placeholder?: string; value?: string }): Promise<string | null> {
  const answer = await showAlert({
    tone: "update",
    title: o.title,
    message: o.message,
    input: { placeholder: o.placeholder, value: o.value },
    actions: [
      { label: t("common.cancel"), value: "\u0000cancel" },
      { label: o.confirm, value: "\u0000ok", primary: true },
    ],
  });
  return answer && answer !== "\u0000cancel" && answer.trim() ? answer.trim() : null;
}

export function closeAlert(value: string | null) {
  const alert = ui.alert;
  ui.alert = queue.shift() ?? null;
  alert?.resolve(value);
}

/** Shows a right-click menu at the pointer. */
export function openMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  ui.menu = { x: e.clientX, y: e.clientY, items };
}

/** Shows a menu under an element (a breadcrumb, a button). */
export function openMenuBelow(el: Element, items: MenuItem[]) {
  const r = el.getBoundingClientRect();
  ui.menu = { x: r.left, y: r.bottom + 4, items };
}

export function closeMenu() {
  ui.menu = null;
}

/** Shows a file or folder in Finder / Explorer. */
export function reveal(path: string) {
  revealItemInDir(path).catch((e) => toast(errorText(e), "error"));
}

/** Copies text; `quiet` skips the "Copied" notice (for Copy and Cut in the editor). */
export async function copyText(text: string, quiet = false) {
  try {
    await navigator.clipboard.writeText(text);
    if (!quiet) toast(t("common.copied"), "ok", undefined, 1800);
  } catch (e) {
    toast(errorText(e), "error");
  }
}

let nextToast = 1;
/** `ms` = 0 keeps the toast until the user closes it or takes its action. */
/** Returns the toast's id, for dismissToast. */
export function toast(text: string, kind: Toast["kind"] = "info", action?: Toast["action"], ms = kind === "error" ? 9000 : 4500): number {
  const id = nextToast++;
  ui.toasts.push({ id, text, kind, action });
  if (ms > 0) setTimeout(() => dismissToast(id), ms);
  return id;
}

export function dismissToast(id: number) {
  ui.toasts = ui.toasts.filter((t) => t.id !== id);
}

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);
export const mod = isMac ? "⌘" : "Ctrl+";
/** A shortcut with Alt / Option: modAlt("G") is "⌘⌥G" or "Ctrl+Alt+G". */
export const modAlt = (key: string) => (isMac ? `⌘⌥${key}` : `Ctrl+Alt+${key}`);
export const isWindows = typeof navigator !== "undefined" && /Win/i.test(navigator.platform || navigator.userAgent);
/** "Reveal in Finder" and its counterparts. */
export const revealLabel = () => t(isMac ? "menu.revealMac" : isWindows ? "menu.revealWindows" : "menu.reveal");
