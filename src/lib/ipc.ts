// Typed calls into the Rust side (src-tauri/src/commands.rs).
import { Channel, invoke } from "@tauri-apps/api/core";

export interface Project {
  root: string;
  name: string;
}
export interface RecentProject extends Project {
  exists: boolean;
}
export interface Entry {
  name: string;
  path: string;
  dir: boolean;
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
  path: string;
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
  recentProjects: () => invoke<RecentProject[]>("recent_projects"),
  forgetRecent: (root: string) => invoke<void>("forget_recent", { root }),
  takeLaunchProject: () => invoke<Project | null>("take_launch_project"),
  watchProject: (root: string) => invoke<void>("watch_project", { root }),
  unwatchProject: (root: string) => invoke<void>("unwatch_project", { root }),

  listDir: (path: string) => invoke<Entry[]>("list_dir", { path }),
  readFile: (path: string) => invoke<FileText>("read_file", { path }),
  listFiles: (root: string) => invoke<FileList>("list_files", { root }),
  searchText: (root: string, query: string, include: string | null) => invoke<TextSearch>("search_text", { root, query, include }),
  writeFile: (path: string, text: string) => invoke<void>("write_file", { path, text }),
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
  checkpointCreate: (root: string, reason: string, note?: string) => invoke<Checkpoint | null>("checkpoint_create", { root, reason, note: note ?? null }),
  checkpointList: (root: string) => invoke<Checkpoint[]>("checkpoint_list", { root }),
  checkpointRestore: (root: string, hash: string) => invoke<void>("checkpoint_restore", { root, hash }),
  checkpointClear: (root: string) => invoke<void>("checkpoint_clear", { root }),
  gitDiscard: (root: string, path: string, from: string | null, untracked: boolean) => invoke<void>("git_discard", { root, path, from, untracked }),
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
  mcpLink: () => invoke<McpLink>("mcp_link"),
  mcpReply: (id: number, result: unknown) => invoke<void>("mcp_reply", { id, result }),

  checkUpdates: () => invoke<UpdateCheck>("check_updates"),
  installUpdate: () => invoke<void>("install_update"),
  setUpdateMode: (mode: string) => invoke<void>("set_update_mode", { mode }),
  pendingUpdate: () => invoke<Release | null>("pending_update"),
  updateChangelog: (version: string, locale: string) => invoke<string>("update_changelog", { version, locale }),
};

export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
