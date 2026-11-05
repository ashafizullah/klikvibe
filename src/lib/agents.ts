// The coding agents KlikVibe knows: the command the terminal's quick button
// types, and how to add KlikVibe's MCP server to each one.
import type { McpInfo } from "./ipc";

/** How an agent takes the MCP server: a command to run once, or a snippet for its config file. */
export type McpSetup = { kind: "command"; text: string } | { kind: "file"; file: string; text: string } | { kind: "none" };

export interface AgentDef {
  id: string;
  label: string;
  /** What starts it in a terminal. */
  command: string;
  /** What starts it again in its last conversation in this folder, when it has that. */
  resume?: string;
  mcp: (info: McpInfo) => McpSetup;
}

const NAME = "klikvibe";
const auth = (info: McpInfo) => `Bearer ${info.token}`;
const json = (value: unknown) => JSON.stringify(value, null, 2);
const cmd = (text: string): McpSetup => ({ kind: "command", text });
const file = (path: string, text: string): McpSetup => ({ kind: "file", file: path, text });

export const AGENTS: AgentDef[] = [
  {
    id: "claude",
    label: "Claude Code",
    command: "claude",
    resume: "claude --continue",
    mcp: (i) => cmd(`claude mcp add --transport http --scope user ${NAME} ${i.url} --header "Authorization: ${auth(i)}"`),
  },
  {
    id: "codex",
    label: "Codex",
    command: "codex",
    resume: "codex resume --last",
    mcp: (i) => file("~/.codex/config.toml", `[mcp_servers.${NAME}]\nurl = "${i.url}"\nhttp_headers = { "Authorization" = "${auth(i)}" }`),
  },
  {
    id: "gemini",
    label: "Gemini CLI",
    command: "gemini",
    resume: "gemini --resume",
    mcp: (i) => cmd(`gemini mcp add --transport http --scope user ${NAME} ${i.url} --header "Authorization: ${auth(i)}"`),
  },
  {
    id: "opencode",
    label: "OpenCode",
    command: "opencode",
    resume: "opencode --continue",
    mcp: (i) =>
      file(
        "~/.config/opencode/opencode.json",
        json({ $schema: "https://opencode.ai/config.json", mcp: { [NAME]: { type: "remote", url: i.url, enabled: true, oauth: false, headers: { Authorization: auth(i) } } } }),
      ),
  },
  {
    id: "omp",
    label: "Oh My Pi",
    command: "omp",
    mcp: (i) => file("~/.omp/agent/mcp.json", json({ mcpServers: { [NAME]: { type: "http", url: i.url, headers: { Authorization: auth(i) } } } })),
  },
  {
    id: "commandcode",
    label: "Command Code",
    command: "cmd",
    mcp: (i) => cmd(`cmd mcp add --transport http --scope user --header "Authorization: ${auth(i)}" ${NAME} ${i.url}`),
  },
  {
    id: "amp",
    label: "Amp",
    command: "amp",
    mcp: (i) => cmd(`amp mcp add ${NAME} --header "Authorization=${auth(i)}" ${i.url}`),
  },
  {
    id: "qwen",
    label: "Qwen Code",
    command: "qwen",
    mcp: (i) => cmd(`qwen mcp add --transport http --scope user ${NAME} ${i.url} --header "Authorization: ${auth(i)}"`),
  },
  {
    id: "kimi",
    label: "Kimi CLI",
    command: "kimi",
    resume: "kimi --continue",
    mcp: (i) => cmd(`kimi mcp add --transport http ${NAME} ${i.url} --header "Authorization: ${auth(i)}"`),
  },
  {
    id: "droid",
    label: "Droid",
    command: "droid",
    mcp: (i) => cmd(`droid mcp add ${NAME} ${i.url} --type http --header "Authorization: ${auth(i)}" --no-oauth`),
  },
  {
    id: "copilot",
    label: "Copilot CLI",
    command: "copilot",
    resume: "copilot --continue",
    mcp: (i) => file("~/.copilot/mcp-config.json", json({ mcpServers: { [NAME]: { type: "http", url: i.url, headers: { Authorization: auth(i) }, tools: ["*"] } } })),
  },
  {
    id: "crush",
    label: "Crush",
    command: "crush",
    mcp: (i) => file("~/.config/crush/crush.json", json({ mcp: { [NAME]: { type: "http", url: i.url, headers: { Authorization: auth(i) } } } })),
  },
  {
    id: "cursor",
    label: "Cursor CLI",
    command: "cursor-agent",
    resume: "cursor-agent resume",
    mcp: (i) => file("~/.cursor/mcp.json", json({ mcpServers: { [NAME]: { url: i.url, headers: { Authorization: auth(i) } } } })),
  },
  {
    id: "aider",
    label: "Aider",
    command: "aider",
    mcp: () => ({ kind: "none" }),
  },
];

export const agentById = (id: string) => AGENTS.find((a) => a.id === id) ?? AGENTS[0];

/** A readable name for the `clientInfo` an agent sent, e.g. "codex-mcp-client 0.50.0" → "Codex". */
export function clientName(client: string | null | undefined): string {
  const known: [RegExp, string][] = [
    [/claude/i, "Claude Code"],
    [/codex/i, "Codex"],
    [/gemini/i, "Gemini CLI"],
    [/opencode/i, "OpenCode"],
    [/oh-my-pi|\bomp\b|\bpi\b/i, "Oh My Pi"],
    [/command.?code/i, "Command Code"],
    [/\bamp\b/i, "Amp"],
    [/qwen/i, "Qwen Code"],
    [/kimi/i, "Kimi CLI"],
    [/droid|factory/i, "Droid"],
    [/copilot/i, "Copilot CLI"],
    [/crush/i, "Crush"],
    [/cursor/i, "Cursor"],
  ];
  if (!client) return "MCP";
  return known.find(([re]) => re.test(client))?.[1] ?? client.split(" ")[0];
}

/** The agent running in a terminal, from its foreground command line ("node /usr/local/bin/claude --resume"). */
export function agentIn(commandLine: string | null): string | null {
  if (!commandLine) return null;
  for (const word of commandLine.split(/\s+/).slice(0, 3)) {
    const name = word.split(/[\\/]/).pop() ?? "";
    const agent = AGENTS.find((a) => a.command === name);
    if (agent) return agent.label;
  }
  return null;
}

const SHELLS = new Set(["zsh", "bash", "fish", "sh", "dash", "ksh", "tcsh", "nu", "pwsh", "powershell", "cmd.exe"]);
const RUNTIMES = new Set(["node", "bun", "deno", "python", "python3", "ruby", "php", "java"]);

/**
 * A short name for a terminal from what runs in front of it: the agent's
 * name, else the command ("npm run dev", "vite", "cargo watch"), or null
 * for the bare shell.
 */
export function terminalName(commandLine: string | null): string | null {
  if (!commandLine) return null;
  const agent = agentIn(commandLine);
  if (agent) return agent;
  const words = commandLine.trim().split(/\s+/);
  const base = (w: string) => w.split(/[\\/]/).pop() ?? w;
  const first = base(words[0]).replace(/^-/, "");
  if (SHELLS.has(first) && words.length === 1) return null;
  // "node /…/bin/vite --port 3000" reads better as "vite --port 3000".
  const rest = RUNTIMES.has(first) && words[1] && !words[1].startsWith("-") ? [base(words[1]), ...words.slice(2)] : [first, ...words.slice(1)];
  const name = rest.slice(0, 3).join(" ");
  return name.length > 24 ? `${name.slice(0, 23)}…` : name;
}
