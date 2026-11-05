// Local MCP server so the user's own coding agent (Claude Code, Codex,
// OpenCode...) can see what the user sees in KlikVibe (the file in front,
// the selection, unsaved edits) and put files and diffs in front of them.
// The server (Rust, mcp.rs) only listens on 127.0.0.1 and needs a secret
// token; tool calls are answered here.
import type { EditorView } from "@codemirror/view";
import { ipc, type McpInfo } from "./ipc";
import { languageName } from "./languages";
import { join, relative } from "./paths";
import { currentProject, isDirty, showProject, workspace, type ProjectState, type Tab } from "./project.svelte";
import { persist, settings } from "./settings.svelte";
import { toast } from "./ui.svelte";

export const mcp = $state({ info: null as McpInfo | null, error: null as string | null });

/** Longest selection or file text a tool returns. */
const MAX_TEXT = 200_000;

const INSTRUCTIONS = [
  "KlikVibe is the code editor the user watches while you work: it shows their project folders, files, Git changes and the terminal you may be running in. Files you change on disk show up there live.",
  "When the user says 'this file', 'this function', 'the selected code' or 'what I'm looking at', call get_editor_state or get_selection first.",
  "read_open_file returns a file as it is in the editor, including edits the user hasn't saved yet. If get_editor_state lists a file as unsaved, read it with read_open_file before you change that file on disk, and tell the user.",
  "To show the user something, use open_file (optionally with line numbers to select those lines) or show_diff to put a changed file's diff in front of them for review. notify shows a short message in the app.",
  "Paths may be absolute or relative to the project in front. Only files inside the projects open in KlikVibe (list_projects) can be opened.",
].join("\n");

interface Tool {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  run: (args: Record<string, unknown>) => string | Promise<string>;
}

const str = (v: unknown, name: string): string => {
  if (typeof v !== "string" || !v.trim()) throw new Error(`\`${name}\` is required.`);
  return v.trim();
};
const num = (v: unknown): number | undefined => (typeof v === "number" && Number.isFinite(v) && v >= 1 ? Math.floor(v) : undefined);
const out = (value: unknown) => JSON.stringify(value, null, 2);
const cap = (text: string) => (text.length > MAX_TEXT ? `${text.slice(0, MAX_TEXT)}\n… (cut off at ${MAX_TEXT} characters)` : text);

function anyProject(): ProjectState {
  const p = currentProject() ?? workspace.projects[0];
  if (!p) throw new Error("No project is open in KlikVibe. Ask the user to open the project folder there.");
  return p;
}

/** A project by root or name; the one in front without `which`. */
function pickProject(which: unknown): ProjectState {
  if (typeof which !== "string" || !which.trim()) return anyProject();
  const p = workspace.projects.find((x) => x.root === which || x.current.name === which);
  if (!p) throw new Error(`No open project matches "${which}". Open projects: ${workspace.projects.map((x) => x.root).join(", ") || "none"}.`);
  return p;
}

/** The open project holding `path`, and the absolute path. */
function locate(path: string): { project: ProjectState; abs: string } {
  if (/^([a-zA-Z]:[\\/]|[\\/])/.test(path)) {
    const project = workspace.projects.filter((p) => relative(p.root, path) !== null).sort((a, b) => b.root.length - a.root.length)[0];
    if (!project) throw new Error(`${path} is outside the projects open in KlikVibe (${workspace.projects.map((p) => p.root).join(", ") || "none"}).`);
    return { project, abs: path.replace(/[\\/]+$/, "") };
  }
  const rel = path.replace(/^\.[\\/]/, "");
  if (rel.split(/[\\/]/).includes("..")) throw new Error("Give a path inside the project, without '..'.");
  const project = anyProject();
  return { project, abs: join(project.root, rel) };
}

function position(view: EditorView, pos: number) {
  const line = view.state.doc.lineAt(pos);
  return { line: line.number, column: pos - line.from + 1 };
}

function selectionOf(view: EditorView) {
  const sel = view.state.selection.main;
  if (sel.empty) return null;
  const start = position(view, sel.from);
  const end = position(view, sel.to);
  return { startLine: start.line, startColumn: start.column, endLine: end.line, endColumn: end.column, text: cap(view.state.sliceDoc(sel.from, sel.to)) };
}

function describeTab(p: ProjectState, tab: Tab) {
  return {
    kind: tab.kind === "file" ? (tab.media ? `${tab.media} preview` : "file") : tab.kind,
    path: tab.kind === "commit" ? undefined : tab.path,
    relativePath: tab.kind === "commit" ? undefined : (relative(p.root, tab.path) ?? undefined),
    commit: tab.kind === "commit" ? tab.id.slice("commit:".length) : tab.kind === "commitFile" ? tab.id.split(":")[1] : undefined,
    unsaved: isDirty(tab),
  };
}

const TOOLS: Tool[] = [
  {
    name: "list_projects",
    description: "Lists the project folders open in KlikVibe (one tab each): root, Git branch, number of changed files, open files and which have unsaved edits.",
    inputSchema: { type: "object", properties: {} },
    run: () =>
      out({
        projects: workspace.projects.map((p) => ({
          name: p.current.name,
          root: p.root,
          inFront: p.root === workspace.active,
          branch: p.git?.branch ?? null,
          changedFiles: p.git?.changes.length ?? 0,
          openFiles: p.tabs.filter((tb) => tb.kind === "file").map((tb) => relative(p.root, tb.path)),
          unsaved: p.tabs.filter(isDirty).map((tb) => relative(p.root, tb.path)),
        })),
      }),
  },
  {
    name: "get_editor_state",
    description:
      "What the user is looking at in a project: the active tab (file, diff or commit), its cursor and selection, and every open tab with its unsaved state. Defaults to the project in front.",
    inputSchema: { type: "object", properties: { project: { type: "string", description: "Project root or name (optional)." } } },
    run: (args) => {
      const p = pickProject(args.project);
      const tab = p.activeTab();
      const view = tab && p.views.get(tab.id);
      return out({
        project: p.root,
        inFront: p.root === workspace.active,
        activeTab: tab
          ? {
              ...describeTab(p, tab),
              language: tab.kind === "file" && !tab.media ? languageName(tab.title) : undefined,
              cursor: view ? position(view, view.state.selection.main.head) : undefined,
              selection: view ? selectionOf(view) : undefined,
            }
          : null,
        openTabs: p.tabs.map((tb) => describeTab(p, tb)),
      });
    },
  },
  {
    name: "get_selection",
    description: "The text the user has selected in the editor (in the project in front), with its file and line range.",
    inputSchema: { type: "object", properties: {} },
    run: () => {
      const p = anyProject();
      const tab = p.activeTab();
      const view = tab && p.views.get(tab.id);
      const sel = view ? selectionOf(view) : null;
      if (!tab || !sel) return "Nothing is selected in KlikVibe.";
      return out({ path: tab.path, relativePath: relative(p.root, tab.path), ...sel });
    },
  },
  {
    name: "read_open_file",
    description: "The text of a file open in KlikVibe as it is in the editor, including edits the user hasn't saved. For files that aren't open, read them from disk yourself.",
    inputSchema: { type: "object", properties: { path: { type: "string", description: "Absolute, or relative to the project in front." } }, required: ["path"] },
    run: (args) => {
      const { project, abs } = locate(str(args.path, "path"));
      const tab = project.tabs.find((tb) => tb.id === `file:${abs}`);
      if (!tab) throw new Error(`${abs} isn't open in KlikVibe; read it from disk instead.`);
      if (tab.media || tab.binary || tab.tooLarge) throw new Error(`${abs} is shown as a preview, not as text.`);
      return out({ path: abs, unsaved: isDirty(tab), text: cap(tab.text) });
    },
  },
  {
    name: "open_file",
    description: "Opens a file in KlikVibe and brings its project to the front, so the user sees it. With `line` (and `endLine`), scrolls there and selects those lines.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute, or relative to the project in front." },
        line: { type: "number", description: "1-based line to show (optional)." },
        endLine: { type: "number", description: "Last line to select (optional)." },
      },
      required: ["path"],
    },
    run: async (args) => {
      const { project, abs } = locate(str(args.path, "path"));
      const line = num(args.line);
      showProject(project.root);
      await project.openFile(abs);
      const tab = project.tabs.find((tb) => tb.id === `file:${abs}`);
      if (!tab) throw new Error(`Couldn't open ${abs}.`);
      if (line && !tab.media) project.revealLines(tab.id, line, num(args.endLine) ?? line);
      return `Opened ${relative(project.root, abs)}${line && !tab.media ? ` at line ${line}` : ""} in KlikVibe.`;
    },
  },
  {
    name: "show_diff",
    description: "Shows the user the uncommitted Git changes to a file in KlikVibe's diff view, for review.",
    inputSchema: { type: "object", properties: { path: { type: "string", description: "Absolute, or relative to the project in front." } }, required: ["path"] },
    run: async (args) => {
      const { project, abs } = locate(str(args.path, "path"));
      const change = project.changeFor(abs);
      if (!change) return `${relative(project.root, abs)} has no uncommitted changes.`;
      showProject(project.root);
      await project.openDiff(change);
      return `Showing the changes to ${change.path} in KlikVibe.`;
    },
  },
  {
    name: "notify",
    description: "Shows a short message to the user in KlikVibe, e.g. that a task is done and what to review.",
    inputSchema: { type: "object", properties: { message: { type: "string", description: "One or two sentences." } }, required: ["message"] },
    run: (args) => {
      const message = str(args.message, "message").slice(0, 400);
      toast(message, "info", undefined, 8000);
      return "Shown.";
    },
  },
];

export async function startMcp() {
  const tools = TOOLS.map(({ name, description, inputSchema }) => ({ name, description, inputSchema }));
  try {
    mcp.info = await ipc.mcpStart(tools, INSTRUCTIONS, async (call) => {
      const tool = TOOLS.find((x) => x.name === call.name);
      try {
        if (!tool) throw new Error(`Unknown tool: ${call.name}`);
        const text = await tool.run(call.arguments ?? {});
        await ipc.mcpReply(call.id, { content: [{ type: "text", text }] });
      } catch (e) {
        await ipc.mcpReply(call.id, { content: [{ type: "text", text: e instanceof Error ? e.message : String(e) }], isError: true });
      }
    });
    mcp.error = null;
    settings.mcpEnabled = true;
  } catch (e) {
    mcp.error = e instanceof Error ? e.message : String(e);
    settings.mcpEnabled = false;
  }
  persist();
}

export async function stopMcp() {
  await ipc.mcpStop().catch(() => {});
  mcp.info = null;
  settings.mcpEnabled = false;
  persist();
}
