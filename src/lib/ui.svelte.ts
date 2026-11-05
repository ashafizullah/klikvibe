// Interface state shared across components: toasts and the busy indicator.

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
  action?: { label: string; run: () => void };
}

export const ui = $state({
  busy: null as string | null,
  toasts: [] as Toast[],
});

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
