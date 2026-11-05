// Layout preferences, kept in the webview's local storage. The recent
// projects list lives on the Rust side, which decides what may be opened.

/** auto: download in the background, install on quit; notify: ask first; off: only on request. */
export type UpdateMode = "auto" | "notify" | "off";

interface Stored {
  showTree: boolean;
  showGit: boolean;
  showTerminal: boolean;
  treeWidth: number;
  gitWidth: number;
  terminalHeight: number;
  /** The agent the terminal's quick button types and the MCP setup is shown for (an id from agents.ts). */
  agent: string;
  /** Run the local MCP server for the user's agent. */
  mcpEnabled: boolean;
  fontSize: number;
  wrap: boolean;
  updateMode: UpdateMode;
  /** The version that last ran, to show what's new once after an update. */
  lastVersion: string;
}

const KEY = "klikvibe.settings";

/** Settings were saved before this run (so this isn't a fresh install). */
export let hadSettings = false;

function load(): Stored {
  const fallback: Stored = {
    showTree: true,
    showGit: true,
    showTerminal: true,
    treeWidth: 250,
    gitWidth: 320,
    terminalHeight: 280,
    agent: "claude",
    mcpEnabled: false,
    fontSize: 13,
    wrap: false,
    updateMode: "auto",
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
