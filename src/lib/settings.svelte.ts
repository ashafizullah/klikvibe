// Layout preferences, kept in the webview's local storage. The recent
// projects list lives on the Rust side, which decides what may be opened.

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
}

const KEY = "klikvibe.settings";

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
  };
  try {
    const raw = localStorage.getItem(KEY);
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
