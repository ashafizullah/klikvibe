// Layout preferences, kept in the webview's local storage. The recent
// projects list lives on the Rust side, which decides what may be opened.

/** auto: download in the background, install on quit; notify: ask first; off: only on request. */
export type UpdateMode = "auto" | "notify" | "off";

/** classic: the terminal under the code; columns: Terminal | Code | Git side by side. */
export type Layout = "classic" | "columns";

interface Stored {
  layout: Layout;
  showTree: boolean;
  /** The file tree in the columns layout: off by default, the editor's tabs are enough there. */
  showTreeColumns: boolean;
  /** The code column: the editor (and the file tree with it). */
  showCode: boolean;
  showGit: boolean;
  showTerminal: boolean;
  treeWidth: number;
  gitWidth: number;
  terminalHeight: number;
  /** The terminal column's width in the columns layout. */
  terminalWidth: number;
  /** The agent the terminal's quick button types and the MCP setup is shown for (an id from agents.ts). */
  agent: string;
  /** Run the local MCP server for the user's agent. */
  mcpEnabled: boolean;
  /** The editor's font size; Mod+= / Mod+- / Mod+0 change it with the terminal's. */
  fontSize: number;
  terminalFontSize: number;
  wrap: boolean;
  updateMode: UpdateMode;
  /** Take checkpoints of the files as they change (see checkpoints in git.rs). */
  checkpoints: boolean;
  /** Checkpoints kept per project, the oldest going first; 0 keeps them all. */
  checkpointLimit: number;
  /** The user's own prompts, under the built-in ones in the terminal's Prompts menu (until promptList is set). */
  prompts: string[];
  /** The whole Prompts menu once the user edited or removed one; null: built-in + prompts (see prompts.svelte.ts). */
  promptList: string[] | null;
  /** Tell the user when an agent in a terminal waits for them (see attention.svelte.ts). */
  notifyAgents: boolean;
  /** Diffs side by side (old left, new right) or in one column. */
  diffView: "split" | "unified";
  /** The history (and its graph) shows every branch, not only the current one. */
  historyAll: boolean;
  /** The version that last ran, to show what's new once after an update. */
  lastVersion: string;
}

const KEY = "klikvibe.settings";

/** Settings were saved before this run (so this isn't a fresh install). */
export let hadSettings = false;

/** The settings of a fresh install (a new object each time). */
export function defaults(): Stored {
  return {
    layout: "classic",
    showTree: true,
    showTreeColumns: false,
    showCode: true,
    showGit: true,
    showTerminal: true,
    treeWidth: 250,
    gitWidth: 320,
    terminalHeight: 280,
    terminalWidth: 560,
    agent: "claude",
    mcpEnabled: false,
    fontSize: 13,
    terminalFontSize: 12.5,
    wrap: false,
    updateMode: "auto",
    historyAll: false,
    diffView: "split",
    notifyAgents: true,
    prompts: [],
    promptList: null,
    checkpoints: true,
    checkpointLimit: 100,
    lastVersion: "",
  };
}

function load(): Stored {
  const fallback = defaults();
  try {
    const raw = localStorage.getItem(KEY);
    hadSettings = !!raw;
    if (raw) return { ...fallback, ...JSON.parse(raw) };
  } catch {}
  return fallback;
}

export const settings = $state<Stored>(load());

/** Mod+= (1), Mod+- (-1) and Mod+0 (0): bigger or smaller text in the editor and the terminal, or back to the default. */
export function zoom(step: -1 | 0 | 1) {
  const d = defaults();
  const clamp = (n: number) => Math.min(28, Math.max(8, n));
  settings.fontSize = step ? clamp(settings.fontSize + step) : d.fontSize;
  settings.terminalFontSize = step ? clamp(settings.terminalFontSize + step) : d.terminalFontSize;
  persist();
}

export function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {}
}
