// The session: which projects were open, their file tabs and what was in
// front, kept in local storage and opened again at the next launch. Folders
// come back through the recent list (open_recent), so one removed from it
// stays closed.
import { ipc } from "./ipc";
import { openProject, showProject, workspace } from "./project.svelte";

const KEY = "klikvibe.session";

interface Saved {
  active: string | null;
  projects: { root: string; files: string[]; activeFile: string | null }[];
}

/** While the last session is being opened again, nothing is saved over it. */
let restoring = false;

function current(): Saved {
  return {
    active: workspace.active,
    projects: workspace.projects.map((p) => ({
      root: p.root,
      files: p.tabs.filter((tb) => tb.kind === "file").map((tb) => tb.path),
      activeFile: p.activeTab()?.kind === "file" ? (p.activeTab()?.path ?? null) : null,
    })),
  };
}

/** Keeps the saved session as it is while projects close for a reason that isn't the user's (an update restarting the app). */
export function holdSession(hold: boolean) {
  restoring = hold;
  if (!hold) saveSession();
}

/** Called from an effect, so it runs whenever projects, tabs or the one in front change. */
export function saveSession() {
  const state = current();
  if (restoring) return;
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {}
}

export async function restoreSession() {
  let saved: Saved | null = null;
  try {
    saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
  } catch {}
  if (!saved?.projects?.length) return;
  restoring = true;
  try {
    for (const s of saved.projects) {
      try {
        await openProject(await ipc.openRecent(s.root));
      } catch {
        continue; // Moved, deleted, or removed from the recent list.
      }
      const p = workspace.projects.find((x) => x.root === s.root);
      if (!p) continue;
      // Files that are gone since are skipped without a message.
      for (const f of s.files) await p.openFile(f, true);
      if (s.activeFile && p.tabs.some((tb) => tb.id === `file:${s.activeFile}`)) p.active = `file:${s.activeFile}`;
    }
    showProject(saved.active && workspace.projects.some((p) => p.root === saved.active) ? saved.active : null);
  } finally {
    restoring = false;
    saveSession();
  }
}
