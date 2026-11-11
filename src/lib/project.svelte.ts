// The open projects, one per tab in the title bar. Each keeps its own file
// tree, open files, Git state and terminals, kept in step with the disk
// while an agent (or anything else) changes files.
import { fromDisk, toDisk, type Eol } from "./eol";
import { getContext, setContext, tick } from "svelte";
import { listen } from "@tauri-apps/api/event";
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import type { EditorView } from "@codemirror/view";
import { errorText, ipc, type Change, type Checkpoint, type Commit, type CommitFile, type Entry, type FsChanged, type GitStatus, type Project } from "./ipc";
import { baseName, dirName, join, relative } from "./paths";
import { mediaKind, type MediaKind } from "./media";
import { forgeName, isMergeRequest, newRequestUrl, parseRemote, type Remote } from "./forge";
import { openUrl } from "@tauri-apps/plugin-opener";
import { t } from "./i18n/index.svelte";
import { confirm, prompt, toast } from "./ui.svelte";
import { persist, settings } from "./settings.svelte";
import { treeKey } from "./grid";
import type { AgentDef } from "./agents";

/** commitFile: one file's changes in a commit; preview: a dev server's page (path is its address). */
export type TabKind = "file" | "diff" | "commit" | "commitFile" | "preview";

export interface Tab {
  id: string;
  kind: TabKind;
  title: string;
  /** The file (file and diff tabs). */
  path: string;
  /** What the editor shows; for diff and commit tabs, the patch. */
  text: string;
  /** The text as last read from or written to disk (with "\n" line ends, as the editor keeps them). */
  saved: string;
  /** The file's line ends, kept when it is saved: CodeMirror turns "\r\n" into "\n". */
  eol: Eol;
  binary: boolean;
  tooLarge: boolean;
  /** The file changed on disk while it had unsaved edits here. */
  changedOnDisk: boolean;
  /** Bumped when the text is replaced from outside the editor (a reload), or a previewed file changes. */
  revision: number;
  loading: boolean;
  /** Shown as a preview (image, video, audio, PDF) instead of as text. */
  media: MediaKind | null;
}

/** How long a file stays marked as "just changed" in the tree. */
const FRESH_MS = 20_000;

export const isDirty = (tab: Tab) => tab.kind === "file" && tab.text !== tab.saved;

const lastLines = (s: string) => s.trim().split("\n").slice(-3).join("\n");

function newTab(kind: TabKind, id: string, title: string, path: string): Tab {
  return { id, kind, title, path, text: "", saved: "", eol: "\n", binary: false, tooLarge: false, changedOnDisk: false, revision: 0, loading: true, media: null };
}

function selectLines(view: EditorView, line: number, endLine: number) {
  const doc = view.state.doc;
  const from = doc.line(Math.min(Math.max(1, line), doc.lines));
  const to = doc.line(Math.min(Math.max(from.number, endLine), doc.lines));
  view.dispatch({ selection: { anchor: from.from, head: to.to }, scrollIntoView: true });
  view.focus();
}

/** Asks before unsaved edits in `tabs` are thrown away (closing them, or their project). */
function confirmDiscard(tabs: Tab[], project?: string): Promise<boolean> {
  const one = tabs.length === 1;
  return confirm({
    title: project ? t("tabs.closeProjectTitle", { name: project }) : one ? t("tabs.discardTitle", { name: tabs[0].title }) : t("tabs.discardManyTitle"),
    message: one ? t("tabs.discardBody", { name: tabs[0].title }) : t("tabs.discardManyBody", { count: tabs.length }),
    confirm: project ? t("tabs.closeWithoutSaving") : t("tabs.discardConfirm"),
    danger: true,
  });
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
  gitTab = $state<"changes" | "history" | "checkpoints">("changes");
  /** Snapshots of the files as an agent works (newest first); null until listed. */
  checkpoints = $state<Checkpoint[] | null>(null);
  /** Why checkpoints can't be taken here (not a repository, a subfolder of one), if so. */
  checkpointsOff = $state<string | null>(null);
  checkpointBusy = $state(false);
  /** Folders the checkpoints leave out (Git repositories of their own), so a restore can't bring them back. */
  checkpointSkipped = $state<string[]>([]);
  /** What the checkpoints take on disk, in bytes; null until measured (or when Git can't say). */
  checkpointBytes = $state<number | null>(null);
  private checkpointTimer: ReturnType<typeof setTimeout> | null = null;
  private changedSince = 0;
  commitMessage = $state("");
  /** The Git action running (commit, push, pull), if any. */
  gitBusy = $state<string | null>(null);
  /** Files changed on disk lately (path → when), shown as fresh in the tree. */
  readonly fresh = new SvelteMap<string, number>();
  /** Every file in the project (relative paths), for search by name; loaded on first search. */
  allFiles = $state.raw<string[] | null>(null);
  allFilesTruncated = $state(false);
  /** The same list as a set, to tell whether a path in terminal output is a file here. */
  private fileSet: Set<string> | null = null;
  private filesLoading = false;
  private filesTimer: ReturnType<typeof setTimeout> | null = null;
  /** The project's terminals and their output, for the MCP tools; set by TerminalPanel. */
  terminals: (() => { index: number; name: string; active: boolean; agent: boolean; tail: (lines: number) => string }[]) | null = null;
  /** Pastes text into the terminal running an agent (else the active one); set by TerminalPanel. */
  askAgent: ((text: string) => void) | null = null;
  /** The terminals as KlikVibe Remote shows them on the phone; set by TerminalPanel. */
  remoteTerminals: (() => { id: string; name: string; agent: string | null }[]) | null = null;
  /** Opens a terminal tab running `agent` (else a plain shell) for the phone; its session id once started. Set by TerminalPanel. */
  newTerminal: ((agent: AgentDef | null) => Promise<string>) | null = null;
  /** Closes the terminal with this session id (KlikVibe Remote's `term.kill`); false if it isn't here. */
  closeTerminal: ((session: string) => boolean) | null = null;
  /** A terminal's scrollback as shown (ANSI) and the output bytes it covers, for KlikVibe Remote. */
  terminalSnapshot: ((session: string, lines: number) => Promise<{ data: string; offset: number } | null> | null) | null = null;
  // ---- what changed since the user last looked ----
  /** When each file last changed on disk (absolute path), not counting the user's own saves. */
  readonly lastChanged = new SvelteMap<string, number>();
  /** When the user last looked at each file (its diff or the file itself). */
  readonly seenAt = new SvelteMap<string, number>();
  /** The user's own saves, so their own edits don't count as news. */
  private ownWrites = new Map<string, number>();
  /** When the Checkpoints tab was last opened: newer checkpoints are new. */
  checkpointsSeen = $state(Date.now());

  /** A change (repository-relative path) the user hasn't looked at since it happened. */
  isUnseen(rel: string): boolean {
    const abs = join(this.root, rel);
    return (this.lastChanged.get(abs) ?? 0) > (this.seenAt.get(abs) ?? 0);
  }

  markSeen(path: string) {
    this.seenAt.set(path, Date.now());
  }

  markAllSeen() {
    const now = Date.now();
    for (const path of this.lastChanged.keys()) this.seenAt.set(path, now);
  }

  /** Where the repository lives on the web (from its remote), for "Open on GitHub" and the like. */
  remote = $state<Remote | null>(null);
  /** The history commit showing its files, if any. */
  expandedCommit = $state<string | null>(null);
  /** The files of commits opened in the history (commits don't change, so they stay). */
  readonly commitFiles = new SvelteMap<string, CommitFile[] | "loading" | "error">();

  /** The editors of file tabs on screen, by tab id (read by the MCP tools). */
  readonly views = new Map<string, EditorView>();
  /** Lines to show once a tab's editor appears (1-based, inclusive). */
  private reveals = new Map<string, { line: number; endLine: number }>();

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
    this.loadRemote();
    // A starting point, so the first change an agent makes can be undone.
    if (settings.checkpoints) this.takeCheckpoint("opened");
    ipc.watchProject(this.root).catch((e) => toast(errorText(e), "error"));
  }

  /** Stops watching; the caller has already confirmed unsaved edits. */
  dispose() {
    this.closed = true;
    if (this.dirTimer) clearTimeout(this.dirTimer);
    if (this.filesTimer) clearTimeout(this.filesTimer);
    if (this.checkpointTimer) clearTimeout(this.checkpointTimer);
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

  async loadRemote() {
    const url = await ipc.gitRemote(this.root).catch(() => null);
    if (!this.closed) this.remote = url ? parseRemote(url) : null;
  }

  // ---- checkpoints ----

  /** A checkpoint once the files have been quiet for 4 s, and at least every minute while they keep changing. */
  private checkpointSoon() {
    // Tried even when they were off for this folder: after `git init` they start by themselves.
    if (!settings.checkpoints) return;
    const now = Date.now();
    this.changedSince ||= now;
    if (this.checkpointTimer) clearTimeout(this.checkpointTimer);
    const wait = now - this.changedSince > 60_000 ? 0 : 4000;
    this.checkpointTimer = setTimeout(() => {
      this.checkpointTimer = null;
      this.changedSince = 0;
      this.takeCheckpoint("changes");
    }, wait);
  }

  /** The new checkpoint, or null when nothing changed (or checkpoints can't be taken here: see checkpointsOff). */
  async takeCheckpoint(reason: string, note?: string): Promise<Checkpoint | null> {
    try {
      const { checkpoint: made, pruned } = await ipc.checkpointCreate(this.root, reason, note, settings.checkpointLimit);
      if (this.closed) return null;
      this.checkpointsOff = null;
      // After a prune the kept ones have new hashes: the list is read again.
      if (pruned) this.loadCheckpoints();
      else if (made && this.checkpoints) {
        this.checkpoints = [made, ...this.checkpoints];
        if (this.gitTab === "checkpoints") this.measureCheckpoints();
      }
      return made;
    } catch (e) {
      // Not a repository (yet), or a subfolder of one: say so in the Checkpoints tab, quietly.
      this.checkpointsOff = errorText(e);
      return null;
    }
  }

  async loadCheckpoints() {
    try {
      this.checkpoints = await ipc.checkpointList(this.root);
      this.measureCheckpoints();
      this.checkpointSkipped = await ipc.checkpointSkipped(this.root).catch(() => []);
    } catch (e) {
      this.checkpointsOff = errorText(e);
    }
  }

  async measureCheckpoints() {
    this.checkpointBytes = await ipc.checkpointSize(this.root).catch(() => null);
  }

  /** A new limit: the checkpoints beyond it go right away. */
  async setCheckpointLimit(keep: number) {
    settings.checkpointLimit = keep;
    persist();
    try {
      if (keep && (await ipc.checkpointPrune(this.root, keep))) await this.loadCheckpoints();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** Puts the files back as they were at `cp`, after asking (the current state is kept as a checkpoint). */
  /** True when the user agreed and the files were put back. */
  async restoreCheckpoint(cp: Checkpoint, when: string): Promise<boolean> {
    const ok = await confirm({
      title: t("checkpoints.restoreTitle", { when }),
      message: t("checkpoints.restoreBody"),
      confirm: t("checkpoints.restore"),
      danger: true,
    });
    if (!ok) return false;
    this.checkpointBusy = true;
    try {
      await ipc.checkpointRestore(this.root, cp.hash);
      toast(t("checkpoints.restored", { when }), "ok");
      return true;
    } catch (e) {
      toast(errorText(e), "error");
      return false;
    } finally {
      this.checkpointBusy = false;
      this.loadCheckpoints();
      this.refreshGit();
    }
  }

  async clearCheckpoints() {
    const ok = await confirm({ title: t("checkpoints.clearTitle"), message: t("checkpoints.clearBody"), confirm: t("checkpoints.clear"), danger: true });
    if (!ok) return;
    try {
      await ipc.checkpointClear(this.root);
      this.checkpoints = [];
      this.checkpointBytes = 0;
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** Lists every file for search by name (again when the disk changed). */
  async loadAllFiles() {
    try {
      const list = await ipc.listFiles(this.root);
      if (this.closed) return;
      this.allFiles = list.files;
      this.fileSet = new Set(list.files);
      this.allFilesTruncated = list.truncated;
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** Whether `rel` (forward slashes) is a file in the project; false until the list is loaded. */
  hasFile(rel: string): boolean {
    if (!this.fileSet && !this.filesLoading) {
      this.filesLoading = true;
      this.loadAllFiles().finally(() => (this.filesLoading = false));
    }
    return this.fileSet?.has(rel) ?? false;
  }
  /** Opens the folders down to `path` in the file tree, shows the tree, and scrolls to it. */
  async revealInTree(path: string, isDir = false) {
    const rel = relative(this.root, path);
    if (rel === null) return;
    const parts = rel.split("/").filter(Boolean);
    let dir = this.root;
    for (const part of isDir ? parts : parts.slice(0, -1)) {
      dir = join(dir, part);
      this.expanded.add(dir);
      if (!this.tree.has(dir)) await this.loadDir(dir);
    }
    settings.showCode = true;
    settings[treeKey(settings)] = true;
    persist();
    await tick();
    document.querySelector(`[data-path="${CSS.escape(path)}"]`)?.scrollIntoView({ block: "center" });
  }

  // ---- new, renamed and deleted files ----

  /** Asks for a name and makes a new file (opened) or folder in `dir`; "src/x.ts" makes the folders too. */
  async newEntry(dir: string, folder: boolean) {
    const where = relative(this.root, dir) || this.current.name;
    const name = await prompt({
      title: folder ? t("files.newFolderTitle") : t("files.newFileTitle"),
      message: t("files.newBody", { where }),
      confirm: t("files.create"),
      placeholder: folder ? "components" : "index.ts",
    });
    if (!name) return;
    try {
      const path = await ipc.createEntry(dir, name, folder);
      await this.revealInTree(path, folder);
      if (!folder) await this.openFile(path);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** The open tabs showing `path` or something inside it (a folder). */
  private tabsUnder(path: string) {
    return this.tabs.filter((tb) => tb.path === path || tb.path.startsWith(path + "/") || tb.path.startsWith(path + "\\"));
  }

  async renameEntry(path: string, isDir: boolean) {
    const old = baseName(path);
    const name = await prompt({ title: t("files.renameTitle", { name: old }), message: t("files.renameBody"), confirm: t("files.rename"), value: old });
    if (!name || name === old) return;
    if (this.tabsUnder(path).some(isDirty)) return toast(t("files.saveFirst"), "error");
    try {
      const to = await ipc.renameEntry(path, name);
      // Open tabs follow the file (or the files in the folder) to the new name.
      for (const tb of this.tabsUnder(path)) {
        const moved = to + tb.path.slice(path.length);
        const wasActive = this.active === tb.id;
        tb.path = moved;
        if (tb.kind === "file") {
          tb.id = `file:${moved}`;
          tb.title = baseName(moved);
        } else if (tb.kind === "diff") {
          // The same id its change opens now, so a click on it comes back to this tab.
          tb.id = `diff:${relative(this.root, moved) ?? moved}`;
          tb.title = baseName(moved);
        }
        if (wasActive) this.active = tb.id;
      }
      await this.loadDir(dirName(path));
      await this.revealInTree(to, isDir);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** `ignored`: .gitignore leaves it out, so no checkpoint can bring it back. */
  async deleteEntry(path: string, isDir: boolean, ignored = false) {
    const name = baseName(path);
    // A checkpoint first, so it can come back from the Checkpoints tab.
    let canUndo = settings.checkpoints && !this.checkpointsOff && !ignored;
    // A Git repository of its own inside the project is in no checkpoint.
    const skipped = canUndo ? await ipc.checkpointSkipped(this.root).catch(() => [] as string[]) : [];
    const rel = relative(this.root, path) ?? "";
    const within = skipped.find((s) => rel === s || rel.startsWith(`${s}/`));
    const holds = skipped.filter((s) => s.startsWith(`${rel}/`));
    if (within) canUndo = false;
    const undo = within
      ? t("files.deleteInRepo", { folder: within })
      : canUndo
        ? t(isDir ? "files.deleteUndoFolder" : "files.deleteUndo") + (holds.length ? ` ${t("files.deleteHoldsRepo", { folders: holds.join(", ") })}` : "")
        : t("files.deleteNoUndo");
    const ok = await confirm({
      title: t("files.deleteTitle", { name }),
      message: `${isDir ? t("files.deleteFolderBody") : t("files.deleteFileBody")} ${undo}`,
      confirm: t("files.delete"),
      danger: true,
    });
    if (!ok) return;
    if (canUndo) await this.takeCheckpoint("before-delete");
    for (const tb of this.tabsUnder(path)) await this.closeTab(tb.id, true);
    try {
      await ipc.deleteEntry(path);
      toast(t("files.deleted", { name }), "ok");
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      this.loadDir(dirName(path));
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

  /** `quiet`: a file that can't be read just closes (restoring a session). */
  private async readInto(tab: Tab, quiet = false) {
    try {
      const file = await ipc.readFile(tab.path);
      tab.binary = file.binary;
      tab.tooLarge = file.tooLarge;
      const disk = fromDisk(file.text ?? "");
      tab.text = tab.saved = disk.text;
      tab.eol = disk.eol;
      tab.changedOnDisk = false;
      tab.revision++;
    } catch (e) {
      if (!quiet) toast(errorText(e), "error");
      this.closeTab(tab.id, true);
    } finally {
      tab.loading = false;
    }
  }

  /** The file changed on disk: show the new text, unless it is what the tab last saved or read
   *  (the echo of a save, or a build touching many files). With unsaved edits, or typing while it
   *  was read, the edits stay and the tab says the file changed. */
  private async syncFromDisk(tab: Tab) {
    const typed = tab.text;
    let file;
    try {
      file = await ipc.readFile(tab.path);
    } catch {
      // Gone (deleted or renamed): an unedited tab closes, as before.
      if (!isDirty(tab)) this.readInto(tab);
      return;
    }
    const disk = fromDisk(file.text ?? "");
    if (!file.binary && !file.tooLarge && disk.text === tab.saved) return;
    if (isDirty(tab) || tab.text !== typed) {
      tab.changedOnDisk = true;
      return;
    }
    tab.binary = file.binary;
    tab.tooLarge = file.tooLarge;
    tab.text = tab.saved = disk.text;
    tab.eol = disk.eol;
    tab.changedOnDisk = false;
    tab.revision++;
  }

  async openFile(path: string, quiet = false) {
    this.markSeen(path);
    const { tab, fresh } = this.show(newTab("file", `file:${path}`, baseName(path), path));
    if (!fresh) return;
    tab.media = mediaKind(path);
    if (tab.media) tab.loading = false;
    else await this.readInto(tab, quiet);
  }

  /** Scrolls a file tab to `line` (through `endLine`) and selects those lines, now or once its editor appears. */
  revealLines(tabId: string, line: number, endLine = line) {
    const view = this.views.get(tabId);
    if (view) selectLines(view, line, endLine);
    else this.reveals.set(tabId, { line, endLine });
  }

  /** Called by the editor when it appears. */
  attachView(tabId: string, view: EditorView) {
    this.views.set(tabId, view);
    const want = this.reveals.get(tabId);
    if (want) {
      this.reveals.delete(tabId);
      selectLines(view, want.line, want.endLine);
    }
  }

  detachView(tabId: string, view: EditorView) {
    if (this.views.get(tabId) === view) this.views.delete(tabId);
  }

  /** Switches a previewed text file (an SVG) to its source. */
  async showSource(tab: Tab) {
    tab.media = null;
    tab.loading = true;
    await this.readInto(tab);
  }

  async openDiff(change: Change) {
    const path = join(this.root, change.path);
    this.markSeen(path);
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

  async openCommit(commit: Pick<Commit, "hash" | "short">) {
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

  /** Shows (or hides) the files a commit changed, under it in the history. */
  async toggleCommitFiles(hash: string) {
    this.expandedCommit = this.expandedCommit === hash ? null : hash;
    if (this.expandedCommit !== hash || this.commitFiles.get(hash) instanceof Array) return;
    this.commitFiles.set(hash, "loading");
    try {
      this.commitFiles.set(hash, await ipc.gitCommitFiles(this.root, hash));
    } catch (e) {
      this.commitFiles.set(hash, "error");
      toast(errorText(e), "error");
    }
  }

  /** A running dev server's page in a tab. */
  openPreview(url: string) {
    const label = url.replace(/^https?:\/\//, "").replace(/\/$/, "");
    const { tab } = this.show(newTab("preview", `preview:${url}`, label, url));
    tab.loading = false;
    // The tab may have gone elsewhere since it was opened: asked for again, it comes back here.
    if (tab.path !== url) {
      tab.path = url;
      tab.revision++;
    }
    showProject(this.root);
  }

  /** One file's changes in a commit, in a tab. */
  async openCommitFile(commit: Pick<Commit, "hash" | "short">, file: CommitFile) {
    const id = `commitfile:${commit.hash}:${file.path}`;
    const { tab, fresh } = this.show(newTab("commitFile", id, `${baseName(file.path)} @ ${commit.short}`, join(this.root, file.local)));
    if (!fresh) return;
    try {
      tab.text = await ipc.gitCommitFileDiff(this.root, commit.hash, file.path, file.from);
      tab.revision++;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      tab.loading = false;
    }
  }

  async closeTab(id: string, force = false) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab) return;
    if (!force && isDirty(tab) && !(await confirmDiscard([tab]))) return;
    // Found again: other tabs (or this one) may have closed while the question was up.
    const i = this.tabs.findIndex((t) => t === tab);
    if (i < 0) return;
    id = tab.id;
    this.tabs.splice(i, 1);
    if (this.active === id) this.active = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null;
  }

  /** Closes several tabs, asking once about unsaved edits among them; `keep` stays in front if it was not closed. */
  async closeTabs(ids: string[], keep?: string) {
    const closing = new Set(ids);
    const dirty = this.tabs.filter((tb) => closing.has(tb.id) && isDirty(tb));
    if (dirty.length && !(await confirmDiscard(dirty))) return;
    const at = this.tabs.findIndex((tb) => tb.id === this.active);
    this.tabs = this.tabs.filter((tb) => !closing.has(tb.id));
    if (this.active && closing.has(this.active)) {
      this.active = keep && this.tabs.some((tb) => tb.id === keep) ? keep : (this.tabs[Math.min(at, this.tabs.length - 1)]?.id ?? null);
    }
  }

  async saveTab(tab: Tab | undefined = this.activeTab()) {
    if (!tab || tab.kind !== "file" || tab.media || tab.binary || tab.tooLarge) return;
    try {
      // What is written counts as saved, not keys typed while it was being written.
      const text = tab.text;
      this.ownWrites.set(tab.path, Date.now());
      await ipc.writeFile(tab.path, toDisk(text, tab.eol));
      tab.saved = text;
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
    if (e.many || e.paths.length) this.checkpointSoon();
    // Files may have come or gone: list them again (soon, once) if a search uses the list.
    if (this.allFiles && !this.filesTimer) this.filesTimer = setTimeout(() => ((this.filesTimer = null), this.loadAllFiles()), 600);
    const now = Date.now();
    const changed = e.many ? null : new Set(e.paths);
    if (changed) {
      for (const p of changed) {
        this.fresh.set(p, now);
        // The echo of the user's own save isn't news.
        if (now - (this.ownWrites.get(p) ?? 0) > 2000) this.lastChanged.set(p, now);
      }
      this.reloadDirsSoon([...changed].map(dirName));
    } else {
      this.reloadDirsSoon(this.tree.keys());
    }
    for (const tab of this.tabs) {
      // A commit, a stage or a discard changes what a diff shows without touching its file.
      if (changed && !changed.has(tab.path) && !(e.git && tab.kind === "diff")) continue;
      if (tab.kind === "file" && tab.media) {
        tab.revision++;
      } else if (tab.kind === "file") {
        this.syncFromDisk(tab);
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
  private gitSeq = 0;

  refreshGit(withLog = false) {
    this.wantLog ||= withLog;
    if (this.gitTimer || this.closed) return;
    this.gitTimer = setTimeout(async () => {
      this.gitTimer = null;
      const log = this.wantLog;
      this.wantLog = false;
      // A refresh started after this one has the newer answer: this one's is dropped.
      const mine = ++this.gitSeq;
      try {
        const status = await ipc.gitStatus(this.root);
        if (this.closed) return;
        if (mine !== this.gitSeq) {
          this.wantLog ||= log;
          return;
        }
        // A remote added or a branch published from the terminal: the web links follow.
        if (status.upstream !== this.git?.upstream) this.loadRemote();
        this.git = status;
        if (log && status.repo) {
          const history = await ipc.gitLog(this.root, settings.historyAll);
          if (mine === this.gitSeq) this.log = history;
          else this.wantLog = true;
        }
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

  /** Commits what is staged, or everything when nothing is. */
  async commit() {
    const message = this.commitMessage.trim();
    if (!message) return toast(t("git.needMessage"), "error");
    const all = !this.git?.changes.some((c) => c.staged);
    if (await this.gitAction("commit", (r) => ipc.gitCommit(r, message, all), t("git.committed"))) this.commitMessage = "";
  }

  async stage(paths: string[]) {
    await ipc.gitStage(this.root, paths).catch((e) => toast(errorText(e), "error"));
    this.refreshGit();
  }

  async unstage(paths: string[]) {
    await ipc.gitUnstage(this.root, paths).catch((e) => toast(errorText(e), "error"));
    this.refreshGit();
  }

  /** Throws away a file's changes (deletes it when Git doesn't track it), after asking. */
  async discard(change: Change) {
    const untracked = change.kind === "untracked";
    const name = baseName(change.path);
    const ok = await confirm({
      title: untracked ? t("git.deleteTitle", { name }) : t("git.discardTitle", { name }),
      message: untracked ? t("git.deleteBody") : t("git.discardBody"),
      confirm: untracked ? t("git.menu.delete") : t("git.menu.discard"),
      danger: true,
    });
    if (!ok) return;
    try {
      await ipc.gitDiscard(this.root, change.path, change.from, untracked);
      // Its diff is now empty; a new file is gone altogether.
      const gone = untracked || change.kind === "added" ? [`file:${join(this.root, change.path)}`] : [];
      for (const id of [`diff:${change.path}`, ...gone]) this.closeTab(id, true);
      toast(untracked ? t("git.deleted") : t("git.discarded"), "ok");
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      this.refreshGit();
    }
  }

  /** Adds a file (or, with `dir`, a folder) to the project's .gitignore. */
  async ignore(rel: string, dir = false) {
    try {
      await ipc.gitIgnore(this.root, rel, dir);
      toast(t("git.ignored", { path: dir ? `${rel}/` : rel }), "ok");
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      this.refreshGit();
    }
  }

  push = async () => {
    const pushed = await this.gitAction("push", ipc.gitPush, t("git.pushed"));
    if (!pushed) return;
    await this.loadRemote();
    // A pushed feature branch is one click from a pull (merge) request.
    const branch = this.git?.branch;
    const url = branch && this.remote && !["main", "master"].includes(branch) ? newRequestUrl(this.remote, branch) : null;
    if (url && branch && this.remote) {
      const label = t(isMergeRequest(this.remote) ? "forge.newMR" : "forge.newPR", { name: forgeName(this.remote) });
      toast(t("forge.pushedBranch", { branch }), "ok", { label, run: () => openUrl(url) }, 12_000);
    }
  };
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

/** Before quitting: asks about the unsaved edits in every project; false when the user keeps them. */
/** The tabs with unsaved edits in every project, each named so it can be told apart later. */
export function unsavedTabs(): string[] {
  return workspace.projects.flatMap((p) => p.tabs.filter(isDirty).map((tb) => `${p.root}\n${tb.id}`));
}

export async function confirmQuit(): Promise<boolean> {
  const dirty = workspace.projects.flatMap((p) => p.tabs.filter(isDirty));
  return !dirty.length || confirmDiscard(dirty);
}

/** Closes a project tab (and its terminals); false when the user keeps unsaved edits. `force`:
 *  the user already chose to throw them away (see `confirmQuit`). */
export async function closeProject(p: ProjectState, force = false): Promise<boolean> {
  const dirty = p.tabs.filter(isDirty);
  if (!force && dirty.length && !(await confirmDiscard(dirty, p.current.name))) return false;
  const i = workspace.projects.indexOf(p);
  if (i < 0) return true;
  workspace.projects.splice(i, 1);
  p.dispose();
  if (workspace.active === p.root) workspace.active = workspace.projects[Math.min(i, workspace.projects.length - 1)]?.root ?? null;
  return true;
}

/** Closes every project, after the user agreed to lose their unsaved edits (`confirmQuit`). */
export async function closeAllProjects(): Promise<void> {
  for (const p of [...workspace.projects]) await closeProject(p, true);
}

/** Moves to the next (+1) or previous (-1) project tab. */
export function cycleProject(dir: 1 | -1) {
  const list = workspace.projects;
  if (!list.length) return;
  const i = list.findIndex((p) => p.root === workspace.active);
  const next = i < 0 ? (dir > 0 ? 0 : list.length - 1) : (i + dir + list.length) % list.length;
  workspace.active = list[next].root;
}

// ---- parallel tasks (git worktrees) ----

/** A new task: its own branch and folder from the current commit, opened as a project. */
export async function newTask(p: ProjectState) {
  const name = await prompt({ title: t("tasks.newTitle"), message: t("tasks.newBody"), confirm: t("tasks.create"), placeholder: t("tasks.placeholder") });
  if (!name) return;
  try {
    const task = await ipc.worktreeAdd(p.root, name);
    await openProject(task);
    toast(t("tasks.ready", { name: task.name }), "ok", undefined, 8000);
  } catch (e) {
    toast(errorText(e), "error", undefined, 0);
  }
}

export async function openTask(p: ProjectState, path: string) {
  try {
    await openProject(await ipc.worktreeOpen(p.root, path));
  } catch (e) {
    toast(errorText(e), "error");
  }
}

/** Merges a task's branch into the main folder's branch, then offers to remove the task. */
export async function mergeTask(p: ProjectState, branch: string, into: string) {
  if (p.git?.changes.length) return toast(t("tasks.commitFirst"), "error", undefined, 9000);
  const ok = await confirm({ title: t("tasks.mergeTitle", { branch, into }), message: t("tasks.mergeBody", { branch, into }), confirm: t("tasks.merge") });
  if (!ok) return;
  try {
    const out = await ipc.worktreeMerge(p.root, branch);
    toast(`${t("tasks.merged", { branch, into })}${out ? `\n${out.trim().split("\n").slice(-2).join("\n")}` : ""}`, "ok");
    workspace.projects.forEach((x) => x.refreshGit(true));
    await removeTask(p, true);
  } catch (e) {
    // Conflicts and the like: Git's own words; they are resolved in the main folder's terminal.
    toast(errorText(e), "error", undefined, 0);
  }
}

/** Removes a task's folder (its branch stays), closing its project first. */
export async function removeTask(p: ProjectState, merged = false) {
  const dirty = !!p.git?.changes.length;
  const ok = await confirm({
    title: t("tasks.removeTitle", { name: p.current.name }),
    message: dirty ? t("tasks.removeDirty") : merged ? t("tasks.removeMerged") : t("tasks.removeBody"),
    confirm: t("tasks.remove"),
    danger: dirty,
  });
  if (!ok) return;
  const root = p.root;
  if (!(await closeProject(p))) return;
  try {
    await ipc.worktreeRemove(root, root, dirty);
    toast(t("tasks.removed"), "ok");
  } catch (e) {
    toast(errorText(e), "error", undefined, 0);
  }
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
