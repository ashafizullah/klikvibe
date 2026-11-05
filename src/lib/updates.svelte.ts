// Updates: the Rust side checks shortly after launch and once a day; a new
// version is offered here and installed only when the user agrees.
import { listen } from "@tauri-apps/api/event";
import { errorText, ipc, type Release } from "./ipc";
import { closeAllProjects } from "./project.svelte";
import { busy, toast } from "./ui.svelte";
import { t } from "./i18n/index.svelte";

export const updates = $state({ checking: false, available: null as Release | null });

function offer(release: Release) {
  updates.available = release;
  toast(t("update.available", { version: release.version }), "ok", { label: t("update.install"), run: installUpdate }, 0);
}

/** A check the user asked for: the outcome is always shown. */
export async function checkForUpdates() {
  if (updates.checking) return;
  updates.checking = true;
  try {
    const result = await ipc.checkUpdates();
    if (result.status === "available") offer(result);
    else if (result.status === "upToDate") toast(t("update.latest", { version: result.current }), "ok");
    else toast(t("update.busy"));
  } catch (e) {
    toast(`${t("update.error")}\n${errorText(e)}`, "error");
  } finally {
    updates.checking = false;
  }
}

/** Closes the projects (asking about unsaved edits), then installs and restarts. */
export async function installUpdate() {
  if (!(await closeAllProjects())) return;
  await busy(t("update.installing"), () => ipc.installUpdate());
}

export function initUpdates() {
  listen<Release>("update-available", (e) => offer(e.payload)).catch(() => {});
}
