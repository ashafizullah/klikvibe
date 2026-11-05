// The open projects, one per tab in the title bar. Each keeps its own file
// tree, open files, Git state and terminals, kept in step with the disk
// while an agent (or anything else) changes files.
import { getContext, setContext } from "svelte";
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

export const isDirty = (tab: Tab) => tab.kind === "file" && tab.text !== tab.saved;

const lastLines = (s: string) => s.trim().split("\n").slice(-3).join("\n");

function newTab(kind: TabKind, id: string, title: string, path: string): Tab {
  return { id, kind, title, path, text: "", saved: "", binary: false, tooLarge: false, changedOnDisk: false, revision: 0, loading: true };
}

/** One open project. */
export class ProjectState {
  readonly current: Project;
  /** Folder path → its entries, for the folders that were opened. */
  readonly tree = new SvelteMap<string, Entry[]>();
  readonly expanded = new SvelteSet<string>();
  tabs = $state<Tab[]>([]);
  active = $state<string | null>(null);
  git = $state<GitStatus | null>(null);
  log = $state<Commit[]>([]);
  gitTab = $state<"changes" | "history">("changes");
  commitMessage = $state("");
  /** The Git action running (commit, push, pull), if any. */
  gitBusy = $state<string | null>(null);
  /** Files changed on disk lately (path → when), shown as fresh in the tree. */
  readonly fresh = new SvelteMap<string, number>();

  private closed = false;
  private pendingDirs = new Set<string>();
  private dirTimer: ReturnType<typeof setTimeout> | null = null;
  private gitTimer: ReturnType<typeof setTimeout> | null = null;
  private wantLog = false;

  constructor(p: Project) {
    this.current = p;
  }

  get root() {
    return this.current.root;
  }

  get dirty() {
    return this.tabs.some(isDirty);
  }

  activeTab(): Tab | undefined {
    return this.tabs.find((t) => t.id === this.active);
  }

  async start() {
    await this.loadDir(this.root);
    this.refreshGit(true);
    ipc.watchProject(this.root).catch((e) => toast(errorText(e), "error"));
  }

  /** Stops watching; the caller has already confirmed unsaved edits. */
  dispose() {
    this.closed = true;
    if (this.dirTimer) clearTimeout(this.dirTimer);
    if (this.gitTimer) clearTimeout(this.gitTimer);
    ipc.unwatchProject(this.root).catch(() => {});
  }

  // ---- tree ----

  async loadDir(path: string) {
    try {
      const entries = await ipc.listDir(path);
      if (!this.closed) this.tree.set(path, entries);
    } catch (e) {
      // A folder that disappeared just drops out of the tree.
      this.tree.delete(path);
      this.expanded.delete(path);
      if (path === this.root) toast(errorText(e), "error");
    }
  }

  toggleDir(path: string) {
    if (this.expanded.has(path)) {
      this.expanded.delete(path);
      return;
    }
    this.expanded.add(path);
    if (!this.tree.has(path)) this.loadDir(path);
  }

  // ---- tabs ----

  /** Opens (or shows) a tab; the returned object is the reactive one in the list. */
  private show(tab: Tab): { tab: Tab; fresh: boolean } {
    const existing = this.tabs.find((t) => t.id === tab.id);
    this.active = tab.id;
    if (existing) return { tab: existing, fresh: false };
    this.tabs.push(tab);
    return { tab: this.tabs[this.tabs.length - 1], fresh: true };
  }

  private async readInto(tab: Tab) {
    try {
      const file = await ipc.readFile(tab.path);
      tab.binary = file.binary;
      tab.tooLarge = file.tooLarge;
      tab.text = tab.saved = file.text ?? "";
      tab.changedOnDisk = false;
      tab.revision++;
    } catch (e) {
      toast(errorText(e), "error");
      this.closeTab(tab.id, true);
    } finally {
      tab.loading = false;
    }
  }

  async openFile(path: string) {
    const { tab, fresh } = this.show(newTab("file", `file:${path}`, baseName(path), path));
    if (fresh) await this.readInto(tab);
  }

  async openDiff(change: Change) {
    const path = join(this.root, change.path);
    const { tab } = this.show(newTab("diff", `diff:${change.path}`, baseName(change.path), path));
    await this.loadDiff(tab, change.path);
  }

  private async loadDiff(tab: Tab, rel: string) {
    try {
      tab.text = await ipc.gitDiff(this.root, rel);
      tab.revision++;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      tab.loading = false;
    }
  }

  async openCommit(commit: Commit) {
    const { tab, fresh } = this.show(newTab("commit", `commit:${commit.hash}`, commit.short, this.root));
    if (!fresh) return;
    try {
      tab.text = await ipc.gitShow(this.root, commit.hash);
      tab.revision++;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      tab.loading = false;
    }
  }

  async closeTab(id: string, force = false) {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    const tab = this.tabs[i];
    if (!force && isDirty(tab) && !(await ask(t("tabs.discard", { name: tab.title }), { title: "KlikVibe", kind: "warning" }))) return;
    this.tabs.splice(i, 1);
    if (this.active === id) this.active = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null;
  }

  async saveTab(tab: Tab | undefined = this.activeTab()) {
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
  reloadTab(tab: Tab) {
    return this.readInto(tab);
  }

  // ---- disk changes ----

  private reloadDirsSoon(dirs: Iterable<string>) {
    for (const d of dirs) this.pendingDirs.add(d);
    this.dirTimer ??= setTimeout(() => {
      const dirs = this.pendingDirs;
      this.pendingDirs = new Set();
      this.dirTimer = null;
      for (const d of dirs) if (this.tree.has(d)) this.loadDir(d);
    }, 150);
  }

  onChanged(e: FsChanged) {
    if (e.git) this.refreshGit(this.gitTab === "history");
    const now = Date.now();
    const changed = e.many ? null : new Set(e.paths);
    if (changed) {
      for (const p of changed) this.fresh.set(p, now);
      this.reloadDirsSoon([...changed].map(dirName));
    } else {
      this.reloadDirsSoon(this.tree.keys());
    }
    for (const tab of this.tabs) {
      if (changed && !changed.has(tab.path)) continue;
      if (tab.kind === "file") {
        if (!isDirty(tab)) this.readInto(tab);
        else tab.changedOnDisk = true;
      } else if (tab.kind === "diff") {
        const rel = relative(this.root, tab.path);
        if (rel) this.loadDiff(tab, rel);
      }
    }
  }

  /** Whether `path` (a file, or a folder holding one) changed in the last moments. */
  isFresh(path: string, now: number): boolean {
    const at = this.fresh.get(path);
    return at !== undefined && now - at < FRESH_MS;
  }

  // ---- git ----

  /** Refreshes the Git status soon (and the history with `withLog`), at most a few times a second. */
  refreshGit(withLog = false) {
    this.wantLog ||= withLog;
    if (this.gitTimer || this.closed) return;
    this.gitTimer = setTimeout(async () => {
      this.gitTimer = null;
      const log = this.wantLog;
      this.wantLog = false;
      try {
        const status = await ipc.gitStatus(this.root);
        if (this.closed) return;
        this.git = status;
        if (log && status.repo) this.log = await ipc.gitLog(this.root);
      } catch (e) {
        if (this.closed) return;
        this.git = null;
        toast(errorText(e), "error");
      }
    }, 200);
  }

  /** A change for the file at `path`, if Git lists one. */
  changeFor(path: string): Change | undefined {
    const rel = relative(this.root, path);
    return rel === null ? undefined : this.git?.changes.find((c) => c.path === rel);
  }

  /** Whether a folder holds any changed file. */
  folderChanged(path: string): boolean {
    const rel = relative(this.root, path);
    if (rel === null || !this.git) return false;
    const prefix = rel ? rel + "/" : "";
    return this.git.changes.some((c) => c.path.startsWith(prefix));
  }

  private async gitAction(label: string, run: (root: string) => Promise<string>, done: string) {
    if (this.gitBusy) return;
    this.gitBusy = label;
    try {
      const out = await run(this.root);
      toast(`${this.current.name}: ${done}` + (out ? `\n${lastLines(out)}` : ""), "ok");
      return true;
    } catch (e) {
      toast(errorText(e), "error", undefined, 0);
      return false;
    } finally {
      this.gitBusy = null;
      this.refreshGit(true);
    }
  }

  async commitAll() {
    const message = this.commitMessage.trim();
    if (!message) return toast(t("git.needMessage"), "error");
    if (await this.gitAction("commit", (r) => ipc.gitCommit(r, message), t("git.committed"))) this.commitMessage = "";
  }

  push = () => this.gitAction("push", ipc.gitPush, t("git.pushed"));
  pull = () => this.gitAction("pull", ipc.gitPull, t("git.pulled"));
  initRepo = () => this.gitAction("init", ipc.gitInit, t("git.initialized"));
}

// ---- the open projects ----

export const workspace = $state({
  projects: [] as ProjectState[],
  /** Root of the project in front; null shows Home. */
  active: null as string | null,
});

export function currentProject(): ProjectState | undefined {
  return workspace.projects.find((p) => p.root === workspace.active);
}

let unlisten: Promise<() => void> | null = null;

/** Opens `p` in a new project tab, or brings its tab to the front. */
export async function openProject(p: Project) {
  workspace.active = p.root;
  if (workspace.projects.some((x) => x.root === p.root)) return;
  const state = new ProjectState(p);
  workspace.projects.push(state);
  unlisten ??= listen<FsChanged>("fs-changed", (e) => workspace.projects.find((x) => x.root === e.payload.root)?.onChanged(e.payload));
  await state.start();
}

export function showProject(root: string | null) {
  workspace.active = root;
}

/** Closes a project tab (and its terminals); false when the user keeps unsaved edits. */
export async function closeProject(p: ProjectState): Promise<boolean> {
  const dirty = p.tabs.filter(isDirty);
  if (dirty.length && !(await ask(`${p.current.name}: ${t("tabs.discardMany", { count: dirty.length })}`, { title: "KlikVibe", kind: "warning" }))) return false;
  const i = workspace.projects.indexOf(p);
  if (i < 0) return true;
  workspace.projects.splice(i, 1);
  p.dispose();
  if (workspace.active === p.root) workspace.active = workspace.projects[Math.min(i, workspace.projects.length - 1)]?.root ?? null;
  return true;
}

/** Closes every project; false when the user keeps unsaved edits in one. */
export async function closeAllProjects(): Promise<boolean> {
  for (const p of [...workspace.projects]) {
    workspace.active = p.root;
    if (!(await closeProject(p))) return false;
  }
  return true;
}

/** Moves to the next (+1) or previous (-1) project tab. */
export function cycleProject(dir: 1 | -1) {
  const list = workspace.projects;
  if (!list.length) return;
  const i = list.findIndex((p) => p.root === workspace.active);
  const next = i < 0 ? (dir > 0 ? 0 : list.length - 1) : (i + dir + list.length) % list.length;
  workspace.active = list[next].root;
}

// ---- the project a component belongs to ----

const KEY = Symbol("project");

export function provideProject(p: ProjectState) {
  setContext(KEY, p);
}

/** The project the calling component is rendered in. */
export function useProject(): ProjectState {
  return getContext<ProjectState>(KEY);
}
