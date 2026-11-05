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
  staged: boolean;
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
  writeFile: (path: string, text: string) => invoke<void>("write_file", { path, text }),
  openExternally: (path: string) => invoke<void>("open_externally", { path }),

  gitStatus: (root: string) => invoke<GitStatus>("git_status", { root }),
  gitLog: (root: string, limit = 200) => invoke<Commit[]>("git_log", { root, limit }),
  gitDiff: (root: string, path: string) => invoke<string>("git_diff", { root, path }),
  gitShow: (root: string, hash: string) => invoke<string>("git_show", { root, hash }),
  gitCommit: (root: string, message: string) => invoke<string>("git_commit", { root, message }),
  gitPush: (root: string) => invoke<string>("git_push", { root }),
  gitPull: (root: string) => invoke<string>("git_pull", { root }),
  gitInit: (root: string) => invoke<string>("git_init", { root }),

  terminalOpen(cwd: string, cols: number, rows: number, onEvent: (e: TerminalEvent) => void) {
    const channel = new Channel<TerminalEvent>();
    channel.onmessage = onEvent;
    return invoke<string>("terminal_open", { cwd, cols, rows, onEvent: channel });
  },
  terminalWrite: (id: string, data: string) => invoke<void>("terminal_write", { id, data }),
  terminalResize: (id: string, cols: number, rows: number) => invoke<void>("terminal_resize", { id, cols, rows }),
  terminalClose: (id: string) => invoke<void>("terminal_close", { id }),

  checkUpdates: () => invoke<UpdateCheck>("check_updates"),
  installUpdate: () => invoke<void>("install_update"),
};

export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
