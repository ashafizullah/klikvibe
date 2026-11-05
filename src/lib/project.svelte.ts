// The open project: its file tree, open tabs and Git state, kept in step
// with the disk while an agent (or anything else) changes files.
import { listen } from "@tauri-apps/api/event";
import { ask } from "@tauri-apps/plugin-dialog";
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { errorText, ipc, type Change, type Commit, type Entry, type FsChanged, type GitStatus, type Project } from "./ipc";
import { baseName, dirName, join, relative } from "./paths";
import { t } from "./i18n/index.svelte";
import { toast } from "./ui.svelte";

export type TabKind = "file" | "diff" | "commit";

export interface Tab {
  id: string;
  kind: TabKind;
  title: string;
  /** The file (file and diff tabs). */
  path: string;
  /** What the editor shows; for diff and commit tabs, the patch. */
  text: string;
  /** The text as last read from or written to disk. */
  saved: string;
  binary: boolean;
  tooLarge: boolean;
  /** The file changed on disk while it had unsaved edits here. */
  changedOnDisk: boolean;
  /** Bumped when the text is replaced from outside the editor (a reload). */
  revision: number;
  loading: boolean;
}

/** How long a file stays marked as "just changed" in the tree. */
const FRESH_MS = 20_000;

export const project = $state({
  current: null as Project | null,
  /** Folder path → its entries, for the folders that were opened. */
  tree: new SvelteMap<string, Entry[]>(),
  expanded: new SvelteSet<string>(),
  tabs: [] as Tab[],
  active: null as string | null,
  git: null as GitStatus | null,
  log: [] as Commit[],
  gitTab: "changes" as "changes" | "history",
  commitMessage: "",
  /** The Git action running (commit, push, pull), if any. */
  gitBusy: null as string | null,
  /** Files changed on disk lately (path → when), shown as fresh in the tree. */
  fresh: new SvelteMap<string, number>(),
});

export function activeTab(): Tab | undefined {
  return project.tabs.find((t) => t.id === project.active);
}

export const isDirty = (tab: Tab) => tab.kind === "file" && tab.text !== tab.saved;

// ---- opening and closing ----

let unlisten: (() => void) | null = null;

export async function openProject(p: Project) {
  if (project.current?.root === p.root) return;
  if (project.current && !(await closeProject())) return;
  project.current = p;
  project.tree.clear();
  project.expanded.clear();
  project.tabs = [];
  project.active = null;
  project.git = null;
  project.log = [];
  project.commitMessage = "";
  project.fresh.clear();
  await loadDir(p.root);
  refreshGit(true);
  ipc.watchProject(p.root).catch((e) => toast(errorText(e), "error"));
  unlisten ??= await listen<FsChanged>("fs-changed", (e) => onChanged(e.payload));
}

/** Back to Home; false when the user keeps unsaved edits. */
export async function closeProject(): Promise<boolean> {
  const dirty = project.tabs.filter(isDirty);
  if (dirty.length && !(await ask(t("tabs.discardMany", { count: dirty.length }), { title: "KlikVibe", kind: "warning" }))) return false;
  const root = project.current?.root;
  project.current = null;
  project.tabs = [];
  project.active = null;
  if (root) ipc.unwatchProject(root).catch(() => {});
  return true;
}

// ---- tree ----

export async function loadDir(path: string) {
  try {
    project.tree.set(path, await ipc.listDir(path));
  } catch (e) {
    // A folder that disappeared just drops out of the tree.
    project.tree.delete(path);
    project.expanded.delete(path);
    if (path === project.current?.root) toast(errorText(e), "error");
  }
}

export function toggleDir(path: string) {
  if (project.expanded.has(path)) {
    project.expanded.delete(path);
    return;
  }
  project.expanded.add(path);
  if (!project.tree.has(path)) loadDir(path);
}

// ---- tabs ----

function newTab(kind: TabKind, id: string, title: string, path: string): Tab {
  return { id, kind, title, path, text: "", saved: "", binary: false, tooLarge: false, changedOnDisk: false, revision: 0, loading: true };
}

/** Opens (or shows) a tab; the returned object is the reactive one in the list. */
function show(tab: Tab): { tab: Tab; fresh: boolean } {
  const existing = project.tabs.find((t) => t.id === tab.id);
  project.active = tab.id;
  if (existing) return { tab: existing, fresh: false };
  project.tabs.push(tab);
  return { tab: project.tabs[project.tabs.length - 1], fresh: true };
}

async function readInto(tab: Tab) {
  try {
    const file = await ipc.readFile(tab.path);
    tab.binary = file.binary;
    tab.tooLarge = file.tooLarge;
    tab.text = tab.saved = file.text ?? "";
    tab.changedOnDisk = false;
    tab.revision++;
  } catch (e) {
    toast(errorText(e), "error");
    closeTab(tab.id, true);
  } finally {
    tab.loading = false;
  }
}

export async function openFile(path: string) {
  const { tab, fresh } = show(newTab("file", `file:${path}`, baseName(path), path));
  if (fresh) await readInto(tab);
}

export async function openDiff(change: Change) {
  const root = project.current?.root;
  if (!root) return;
  const path = join(root, change.path);
  const { tab } = show(newTab("diff", `diff:${change.path}`, baseName(change.path), path));
  await loadDiff(tab, change.path);
}

async function loadDiff(tab: Tab, rel: string) {
  const root = project.current?.root;
  if (!root) return;
  try {
    tab.text = await ipc.gitDiff(root, rel);
    tab.revision++;
  } catch (e) {
    toast(errorText(e), "error");
  } finally {
    tab.loading = false;
  }
}

export async function openCommit(commit: Commit) {
  const root = project.current?.root;
  if (!root) return;
  const { tab, fresh } = show(newTab("commit", `commit:${commit.hash}`, commit.short, root));
  if (!fresh) return;
  try {
    tab.text = await ipc.gitShow(root, commit.hash);
    tab.revision++;
  } catch (e) {
    toast(errorText(e), "error");
  } finally {
    tab.loading = false;
  }
}

export async function closeTab(id: string, force = false) {
  const i = project.tabs.findIndex((t) => t.id === id);
  if (i < 0) return;
  const tab = project.tabs[i];
  if (!force && isDirty(tab) && !(await ask(t("tabs.discard", { name: tab.title }), { title: "KlikVibe", kind: "warning" }))) return;
  project.tabs.splice(i, 1);
  if (project.active === id) project.active = project.tabs[Math.min(i, project.tabs.length - 1)]?.id ?? null;
}

export async function saveTab(tab: Tab | undefined = activeTab()) {
  if (!tab || tab.kind !== "file" || tab.binary || tab.tooLarge) return;
  try {
    await ipc.writeFile(tab.path, tab.text);
    tab.saved = tab.text;
    tab.changedOnDisk = false;
  } catch (e) {
    toast(errorText(e), "error");
  }
}

/** Throws away local edits and shows what is on disk. */
export function reloadTab(tab: Tab) {
  return readInto(tab);
}

// ---- disk changes ----

let pendingDirs = new Set<string>();
let dirTimer: ReturnType<typeof setTimeout> | null = null;

function reloadDirsSoon(dirs: Iterable<string>) {
  for (const d of dirs) pendingDirs.add(d);
  dirTimer ??= setTimeout(() => {
    const dirs = pendingDirs;
    pendingDirs = new Set();
    dirTimer = null;
    for (const d of dirs) if (project.tree.has(d)) loadDir(d);
  }, 150);
}

function onChanged(e: FsChanged) {
  const root = project.current?.root;
  if (!root || e.root !== root) return;
  if (e.git) refreshGit(project.gitTab === "history");
  const now = Date.now();
  const changed = e.many ? null : new Set(e.paths);
  if (changed) {
    for (const p of changed) project.fresh.set(p, now);
    reloadDirsSoon([...changed].map(dirName));
  } else {
    reloadDirsSoon(project.tree.keys());
  }
  for (const tab of project.tabs) {
    if (changed && !changed.has(tab.path)) continue;
    if (tab.kind === "file") {
      if (!isDirty(tab)) readInto(tab);
      else tab.changedOnDisk = true;
    } else if (tab.kind === "diff") {
      const rel = relative(root, tab.path);
      if (rel) loadDiff(tab, rel);
    }
  }
}

/** Whether `path` (a file, or a folder holding one) changed in the last moments. */
export function isFresh(path: string, now: number): boolean {
  const at = project.fresh.get(path);
  return at !== undefined && now - at < FRESH_MS;
}

// ---- git ----

let gitTimer: ReturnType<typeof setTimeout> | null = null;
let wantLog = false;

/** Refreshes the Git status soon (and the history with `withLog`), at most a few times a second. */
export function refreshGit(withLog = false) {
  wantLog ||= withLog;
  if (gitTimer) return;
  gitTimer = setTimeout(async () => {
    gitTimer = null;
    const root = project.current?.root;
    if (!root) return;
    const log = wantLog;
    wantLog = false;
    try {
      const status = await ipc.gitStatus(root);
      if (project.current?.root !== root) return;
      project.git = status;
      if (log && status.repo) project.log = await ipc.gitLog(root);
    } catch (e) {
      project.git = null;
      toast(errorText(e), "error");
    }
  }, 200);
}

/** A change for the file at `path`, if Git lists one. */
export function changeFor(path: string): Change | undefined {
  const root = project.current?.root;
  const rel = root ? relative(root, path) : null;
  return rel === null ? undefined : project.git?.changes.find((c) => c.path === rel);
}

/** Whether a folder holds any changed file. */
export function folderChanged(path: string): boolean {
  const root = project.current?.root;
  const rel = root ? relative(root, path) : null;
  if (rel === null || !project.git) return false;
  const prefix = rel ? rel + "/" : "";
  return project.git.changes.some((c) => c.path.startsWith(prefix));
}

async function gitAction(label: string, run: (root: string) => Promise<string>, done: string) {
  const root = project.current?.root;
  if (!root || project.gitBusy) return;
  project.gitBusy = label;
  try {
    const out = await run(root);
    toast(done + (out ? `\n${lastLines(out)}` : ""), "ok");
    return true;
  } catch (e) {
    toast(errorText(e), "error", undefined, 0);
    return false;
  } finally {
    project.gitBusy = null;
    refreshGit(true);
  }
}

const lastLines = (s: string) => s.trim().split("\n").slice(-3).join("\n");

export async function commitAll() {
  const message = project.commitMessage.trim();
  if (!message) return toast(t("git.needMessage"), "error");
  if (await gitAction("commit", (r) => ipc.gitCommit(r, message), t("git.committed"))) project.commitMessage = "";
}

export const push = () => gitAction("push", ipc.gitPush, t("git.pushed"));
export const pull = () => gitAction("pull", ipc.gitPull, t("git.pulled"));
export const initRepo = () => gitAction("init", ipc.gitInit, t("git.initialized"));
