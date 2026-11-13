// Typed calls into the Rust side (src-tauri/src/commands.rs).
import { Channel, invoke } from "@tauri-apps/api/core";

export interface Project {
  root: string;
  name: string;
}
export interface RecentProject extends Project {
  exists: boolean;
}
export type HunkAction = "stage" | "unstage" | "discard";

export interface Entry {
  name: string;
  path: string;
  dir: boolean;
  /** Left out by .gitignore: shown dimmed. */
  ignored: boolean;
}
export interface FileList {
  files: string[];
  truncated: boolean;
}
export interface TextMatch {
  path: string;
  line: number;
  text: string;
  /** The match's byte range in `text` (equal to the character range for ASCII). */
  start: number;
  end: number;
}
export interface TextSearch {
  matches: TextMatch[];
  truncated: boolean;
}
export interface FileText {
  text: string | null;
  size: number;
  binary: boolean;
  tooLarge: boolean;
}
export type ChangeKind = "added" | "modified" | "deleted" | "renamed" | "untracked" | "conflict";
export interface Change {
  path: string;
  from: string | null;
  kind: ChangeKind;
  /** Some of it is staged (in the next commit). */
  staged: boolean;
  /** Some of it isn't: a partly staged file is both. */
  unstaged: boolean;
}
export interface GitStatus {
  repo: boolean;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  changes: Change[];
}
export interface Commit {
  hash: string;
  short: string;
  author: string;
  date: string;
  subject: string;
  local: boolean;
  parents: string[];
  /** Full decorations, e.g. "HEAD -> refs/heads/main" (see refLabels in graph.ts). */
  refs: string[];
}
export interface CommitFile {
  /** From the repository's top, as the commit names it. */
  path: string;
  /** The same file from the project folder (it may be a folder inside the repository). */
  local: string;
  from: string | null;
  kind: "added" | "modified" | "deleted" | "renamed";
  /** Lines added and removed; null for binary files. */
  added: number | null;
  removed: number | null;
}
export interface Checkpoint {
  hash: string;
  short: string;
  date: string;
  /** "opened", "changes", "manual", "agent", "before-restore" or "restored". */
  reason: string;
  note: string;
  files: number;
}
export interface Worktree {
  path: string;
  branch: string | null;
  /** The repository's main folder. */
  main: boolean;
}
export interface FsChanged {
  root: string;
  paths: string[];
  many: boolean;
  git: boolean;
}
export interface Release {
  version: string;
  current: string;
  notes: string;
}
export type UpdateCheck = ({ status: "available" } & Release) | { status: "upToDate"; current: string } | { status: "busy" };
export interface RemotePhone {
  id: string;
  name: string;
  platform: string | null;
  connected: boolean;
}
export interface RemoteStatus {
  enabled: boolean;
  port: number | null;
  addresses: string[];
  /** Added to an account by a phone (it has a token for the KlikVibe server). */
  linked: boolean;
  account: string | null;
  pro: boolean;
  /** Connected to the relay (Pro): phones reach this computer from anywhere. */
  relay: boolean;
  /** The relay's last connection crashed (a bug); it is tried again every minute. */
  relayError: string | null;
  phones: RemotePhone[];
  pairing: { url: string; svg: string; expiresAt: string } | null;
  /** A phone scanned the code: it is added only if the user allows it. */
  /** `check`: the code the phone that scanned shows; the user compares the two. */
  confirm: { phone: string; platform: string | null; account: string | null; check: string } | null;
  error: string | null;
}
/** A project as the phone sees it (src-tauri/src/remote/hub.rs). */
export interface RemoteProject {
  root: string;
  name: string;
  attention: boolean;
  terminals: { id: string; name: string; agent: string | null }[];
}

export interface McpCall {
  id: number;
  name: string;
  arguments: Record<string, unknown>;
}
export interface McpInfo {
  url: string;
  token: string;
}
export interface McpLink {
  running: boolean;
  connected: boolean;
  client: string | null;
  idleSecs: number | null;
}
export type TerminalEvent = { type: "data"; data: string } | { type: "exit"; code: number | null };

export const ipc = {
  pickProject: () => invoke<Project | null>("pick_project"),
  openRecent: (root: string) => invoke<Project>("open_recent", { root }),
  homeDir: () => invoke<string | null>("home_dir"),
  /** A path dropped on the window: a file to open, or a folder now open as a project. */
  openDropped: (path: string) => invoke<{ kind: "file"; path: string } | { kind: "folder"; project: Project }>("open_dropped", { path }),
  recentProjects: () => invoke<RecentProject[]>("recent_projects"),
  forgetRecent: (root: string) => invoke<void>("forget_recent", { root }),
  takeLaunchProject: () => invoke<Project | null>("take_launch_project"),
  watchProject: (root: string) => invoke<void>("watch_project", { root }),
  unwatchProject: (root: string) => invoke<void>("unwatch_project", { root }),

  listDir: (path: string) => invoke<Entry[]>("list_dir", { path }),
  /** "file", "dir" or null (missing, or outside the open projects), per path. */
  pathKinds: (paths: string[]) => invoke<("file" | "dir" | null)[]>("path_kinds", { paths }),
  /** Absolute paths outside the open projects: "media", "file", "dir" or null (missing), per path. */
  outsideKinds: (paths: string[]) => invoke<("media" | "file" | "dir" | null)[]>("outside_kinds", { paths }),
  /** Media in its default app, anything else shown in Finder/Explorer. */
  openOutside: (path: string) => invoke<void>("open_outside", { path }),
  readFile: (path: string) => invoke<FileText>("read_file", { path }),
  listFiles: (root: string) => invoke<FileList>("list_files", { root }),
  /** Lines that look like they define `name` (go to definition, without a language server). */
  findDefinitions: (root: string, name: string) => invoke<TextMatch[]>("find_definitions", { root, name }),
  searchText: (root: string, query: string, include: string | null) => invoke<TextSearch>("search_text", { root, query, include }),
  writeFile: (path: string, text: string) => invoke<void>("write_file", { path, text }),
  /** Returns the new file's or folder's path. */
  createEntry: (dir: string, name: string, folder: boolean) => invoke<string>("create_entry", { dir, name, folder }),
  renameEntry: (path: string, name: string) => invoke<string>("rename_entry", { path, name }),
  deleteEntry: (path: string) => invoke<void>("delete_entry", { path }),
  openExternally: (path: string) => invoke<void>("open_externally", { path }),
  clipboardText: () => invoke<string>("clipboard_text"),

  gitStatus: (root: string) => invoke<GitStatus>("git_status", { root }),
  gitLog: (root: string, all: boolean, limit = 200) => invoke<Commit[]>("git_log", { root, limit, all }),
  gitDiff: (root: string, path: string) => invoke<string>("git_diff", { root, path }),
  gitShow: (root: string, hash: string) => invoke<string>("git_show", { root, hash }),
  gitCommitFiles: (root: string, hash: string) => invoke<CommitFile[]>("git_commit_files", { root, hash }),
  gitCommitFileDiff: (root: string, hash: string, path: string, from: string | null) => invoke<string>("git_commit_file_diff", { root, hash, path, from }),
  /** `all`: stage everything first; otherwise only what is staged is committed. */
  gitCommit: (root: string, message: string, all: boolean) => invoke<string>("git_commit", { root, message, all }),
  gitStage: (root: string, paths: string[]) => invoke<void>("git_stage", { root, paths }),
  gitUnstage: (root: string, paths: string[]) => invoke<void>("git_unstage", { root, paths }),
  gitPush: (root: string) => invoke<string>("git_push", { root }),
  gitPull: (root: string) => invoke<string>("git_pull", { root }),
  gitInit: (root: string) => invoke<string>("git_init", { root }),
  gitRemote: (root: string) => invoke<string | null>("git_remote", { root }),
  worktreeList: (root: string) => invoke<Worktree[]>("worktree_list", { root }),
  worktreeAdd: (root: string, name: string) => invoke<Project>("worktree_add", { root, name }),
  worktreeOpen: (root: string, path: string) => invoke<Project>("worktree_open", { root, path }),
  worktreeMerge: (root: string, branch: string) => invoke<string>("worktree_merge", { root, branch }),
  worktreeRemove: (root: string, path: string, force: boolean) => invoke<void>("worktree_remove", { root, path, force }),
  /** `keep`: at most about this many are kept (0: all); `pruned`: older ones went, so the list has new hashes. */
  checkpointCreate: (root: string, reason: string, note: string | undefined, keep: number) =>
    invoke<{ checkpoint: Checkpoint | null; pruned: boolean }>("checkpoint_create", { root, reason, note: note ?? null, keep }),
  checkpointList: (root: string, limit = 1000) => invoke<Checkpoint[]>("checkpoint_list", { root, limit }),
  checkpointPrune: (root: string, keep: number) => invoke<boolean>("checkpoint_prune", { root, keep }),
  /** Bytes on disk; null when Git is too old to say. */
  /** Folders the checkpoints leave out: Git repositories of their own inside the project. */
  checkpointSkipped: (root: string) => invoke<string[]>("checkpoint_skipped", { root }),
  checkpointSize: (root: string) => invoke<number | null>("checkpoint_size", { root }),
  checkpointRestore: (root: string, hash: string) => invoke<void>("checkpoint_restore", { root, hash }),
  checkpointClear: (root: string) => invoke<void>("checkpoint_clear", { root }),
  gitDiscard: (root: string, path: string, from: string | null, untracked: boolean) => invoke<void>("git_discard", { root, path, from, untracked }),
  /** For each hunk of a file's changes (against the last commit), whether it is staged. */
  gitStagedHunks: (root: string, path: string) => invoke<boolean[]>("git_staged_hunks", { root, path }),
  /** Stages, unstages or throws away one hunk, given as shown. */
  /** The patch it applied; for a discard, `gitHunkRestore` puts it back. */
  gitHunk: (root: string, path: string, hunk: string, action: HunkAction) => invoke<string>("git_hunk", { root, path, hunk, action }),
  gitHunkRestore: (root: string, patch: string) => invoke<void>("git_hunk_restore", { root, patch }),
  gitIgnore: (root: string, path: string, dir: boolean) => invoke<void>("git_ignore", { root, path, dir }),

  terminalOpen(cwd: string, cols: number, rows: number, onEvent: (e: TerminalEvent) => void) {
    const channel = new Channel<TerminalEvent>();
    channel.onmessage = onEvent;
    return invoke<string>("terminal_open", { cwd, cols, rows, onEvent: channel });
  },
  terminalWrite: (id: string, data: string) => invoke<void>("terminal_write", { id, data }),
  terminalResize: (id: string, cols: number, rows: number) => invoke<void>("terminal_resize", { id, cols, rows }),
  terminalClose: (id: string) => invoke<void>("terminal_close", { id }),
  /** The command line of the program in front in a terminal (null on Windows). */
  terminalForeground: (id: string) => invoke<string | null>("terminal_foreground", { id }),

  mcpStart(tools: unknown, instructions: string, onCall: (c: McpCall) => void) {
    const channel = new Channel<McpCall>();
    channel.onmessage = onCall;
    return invoke<McpInfo>("mcp_start", { tools, instructions, onCall: channel });
  },
  mcpStop: () => invoke<void>("mcp_stop"),
  remoteStatus: () => invoke<RemoteStatus>("remote_status"),
  remoteSetEnabled: (on: boolean) => invoke<RemoteStatus>("remote_set_enabled", { on }),
  remotePairStart: () => invoke<RemoteStatus>("remote_pair_start"),
  remotePairCancel: () => invoke<RemoteStatus>("remote_pair_cancel"),
  remotePairConfirm: (allow: boolean) => invoke<RemoteStatus>("remote_pair_confirm", { allow }),
  remoteForgetPhone: (id: string) => invoke<RemoteStatus>("remote_forget_phone", { id }),
  remoteUnlink: () => invoke<RemoteStatus>("remote_unlink"),
  remoteProjects: (projects: RemoteProject[]) => invoke<void>("remote_projects", { projects }),
  remoteAttention: (root: string, term: string, title: string, body: string, kind: string) =>
    invoke<void>("remote_attention", { root, term, title, body, kind }),
  remoteAgents: (agents: { id: string; label: string }[]) => invoke<void>("remote_agents", { agents }),
  remotePrompts: (prompts: string[]) => invoke<void>("remote_prompts", { prompts }),
  remoteTermCreated: (nonce: string, id: string | null, error: string | null) => invoke<void>("remote_term_created", { nonce, id, error }),
  remoteTermSnapshot: (nonce: string, data: string | null, offset: number) => invoke<void>("remote_term_snapshot", { nonce, data, offset }),
  mcpLink: () => invoke<McpLink>("mcp_link"),
  mcpReply: (id: number, result: unknown) => invoke<void>("mcp_reply", { id, result }),

  checkUpdates: () => invoke<UpdateCheck>("check_updates"),
  installUpdate: () => invoke<void>("install_update"),
  setUpdateMode: (mode: string) => invoke<void>("set_update_mode", { mode }),
  pendingUpdate: () => invoke<Release | null>("pending_update"),
  updateChangelog: (version: string, locale: string) => invoke<string>("update_changelog", { version, locale }),
  prepareUpdate: () => invoke<void>("prepare_update"),
  setUnsaved: (unsaved: boolean) => invoke<void>("set_unsaved", { unsaved }),
  quitApp: () => invoke<void>("quit_app"),
};

export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
