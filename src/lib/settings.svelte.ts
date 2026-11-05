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
  fontSize: number;
  wrap: boolean;
  updateMode: UpdateMode;
  /** Take checkpoints of the files as they change (see checkpoints in git.rs). */
  checkpoints: boolean;
  /** The user's own prompts for the terminal's Prompts menu. */
  prompts: string[];
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

function load(): Stored {
  const fallback: Stored = {
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
    wrap: false,
    updateMode: "auto",
    historyAll: false,
    diffView: "split",
    notifyAgents: true,
    prompts: [],
    checkpoints: true,
    lastVersion: "",
  };
  try {
    const raw = localStorage.getItem(KEY);
    hadSettings = !!raw;
    if (raw) return { ...fallback, ...JSON.parse(raw) };
  } catch {}
  return fallback;
}

export const settings = $state<Stored>(load());

export function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {}
}
