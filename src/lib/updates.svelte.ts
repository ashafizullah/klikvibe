// Updates: the Rust side checks and downloads as the user's choice says
// (About & updates); every message about them is shown here, with the
// changelog in the interface language.
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { errorText, ipc, type Release } from "./ipc";
import { closeAllProjects, confirmQuit } from "./project.svelte";
import { holdSession, restoreSession } from "./session.svelte";
import { hadSettings, persist, settings, type UpdateMode } from "./settings.svelte";
import { dismissToast, showAlert, toast } from "./ui.svelte";
import { changelogSection } from "./changelog";
import { i18n, t } from "./i18n/index.svelte";
import changelogEn from "../../CHANGELOG.md?raw";
import changelogId from "../../CHANGELOG.id.md?raw";

export const updates = $state({
  checking: false,
  /** Downloading and installing (then the app restarts). */
  installing: false,
  /** Downloaded in the background, installed on quit (auto mode). */
  ready: null as Release | null,
  /** Found by a check, not downloaded yet. */
  available: null as Release | null,
  /** The outcome of the last check the user asked for, shown in About. */
  lastResult: null as null | { kind: "latest" | "error"; text: string },
});

/** The running version's changes, from the changelog shipped with the app. */
function bundledNotes(version: string): string {
  return changelogSection(i18n.locale === "id" ? changelogId : changelogEn, version) || changelogSection(changelogEn, version);
}

/** A newer version's changes in the interface language, from the repository at its tag; the (English) release notes if that fails. */
async function releaseNotes(release: Release): Promise<string> {
  try {
    return changelogSection(await ipc.updateChangelog(release.version, i18n.locale), release.version) || release.notes;
  } catch {
    return release.notes;
  }
}

/** Shows what a new version brings and installs it when the user agrees. */
export async function offer(release: Release) {
  updates.available = release;
  const downloaded = updates.ready?.version === release.version;
  const choice = await showAlert({
    tone: "update",
    title: t("update.availableTitle"),
    message: t("update.availableBody", { version: release.version, current: release.current }),
    markdown: await releaseNotes(release),
    actions: [
      { label: t("update.later"), value: "later" },
      { label: t(downloaded ? "update.restart" : "update.install"), value: "install", primary: true },
    ],
  });
  if (choice === "install") await installUpdate();
}

/** A check the user asked for: the outcome is always shown. */
export async function checkForUpdates() {
  if (updates.checking) return;
  updates.checking = true;
  updates.lastResult = null;
  try {
    const result = await ipc.checkUpdates();
    if (result.status === "available") {
      updates.checking = false;
      await offer(result);
    } else if (result.status === "upToDate") {
      updates.lastResult = { kind: "latest", text: t("update.latestBody", { version: result.current }) };
      toast(t("update.latestBody", { version: result.current }), "ok");
    } else toast(t("update.busy"));
  } catch (e) {
    updates.lastResult = { kind: "error", text: errorText(e) };
    await showAlert({ tone: "error", title: t("update.errorTitle"), message: errorText(e) });
  } finally {
    updates.checking = false;
  }
}

/** Asks about unsaved edits (every project at once, before anything closes), downloads the update
 *  with the projects still open, then closes them and installs and restarts. A failure on the way
 *  leaves the projects as they were, or opens them again. */
export async function installUpdate() {
  if (updates.installing) return;
  if (!(await confirmQuit())) return;
  updates.installing = true;
  // Stays until KlikVibe restarts into the new version.
  const notice = toast(t("update.installing"), "info", undefined, 0);
  let closed = false;
  try {
    await ipc.prepareUpdate();
    // The projects close before the restart, but open again after it (the session isn't saved empty).
    holdSession(true);
    closed = true;
    await closeAllProjects();
    await ipc.installUpdate();
  } catch (e) {
    dismissToast(notice);
    if (closed) {
      await restoreSession();
      holdSession(false);
    }
    await showAlert({ tone: "error", title: t("update.installFailed"), message: errorText(e) });
  } finally {
    updates.installing = false;
  }
}

function announce(release: Release) {
  updates.ready = release;
  toast(t("update.ready", { version: release.version }), "ok", { label: t("update.whatsNew"), run: () => offer(release) }, 0);
}

/** After an update, once: what changed in the version now running. */
async function showWhatsNew() {
  const current = await getVersion();
  const last = settings.lastVersion;
  if (last === current) return;
  settings.lastVersion = current;
  persist();
  // A fresh install has nothing to compare with. 0.1.0 didn't record
  // lastVersion, so saved settings without one mean an update from it.
  const notes = last || hadSettings ? bundledNotes(current) : "";
  if (notes) await showAlert({ tone: "update", title: t("update.whatsNewTitle", { version: current }), message: t("update.whatsNewBody"), markdown: notes });
}

/** The running version's changes, on request (About). */
export async function showCurrentNotes() {
  const current = await getVersion();
  await showAlert({ tone: "update", title: t("update.whatsNewTitle", { version: current }), message: t("about.thisVersionBody"), markdown: bundledNotes(current) || "—" });
}

/** Sends the saved choice to the backend and listens for what it finds. */
export async function initUpdates() {
  await ipc.setUpdateMode(settings.updateMode).catch(() => {});
  listen<Release>("update-ready", (e) => announce(e.payload)).catch(() => {});
  listen<Release>("update-available", (e) => offer(e.payload)).catch(() => {});
  const ready = await ipc.pendingUpdate().catch(() => null);
  if (ready) announce(ready);
  await showWhatsNew();
}

export async function setUpdateMode(mode: UpdateMode) {
  settings.updateMode = mode;
  persist();
  await ipc.setUpdateMode(mode);
  // Leaving "auto" drops a downloaded update (the backend won't install it on quit).
  if (mode !== "auto") updates.ready = null;
}
