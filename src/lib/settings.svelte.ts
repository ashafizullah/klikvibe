// Layout preferences, kept in the webview's local storage. The recent
// projects list lives on the Rust side, which decides what may be opened.

export type Agent = "claude" | "codex" | "gemini";

export const AGENTS: { id: Agent; label: string; command: string }[] = [
  { id: "claude", label: "Claude Code", command: "claude" },
  { id: "codex", label: "Codex", command: "codex" },
  { id: "gemini", label: "Gemini CLI", command: "gemini" },
];

interface Stored {
  showTree: boolean;
  showGit: boolean;
  showTerminal: boolean;
  treeWidth: number;
  gitWidth: number;
  terminalHeight: number;
  /** The agent the terminal's quick button types. */
  agent: Agent;
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
