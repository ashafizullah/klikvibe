// Local MCP server so the user's own coding agent (Claude Code, Codex,
// OpenCode...) can see what the user sees in KlikVibe (the file in front,
// the selection, unsaved edits, the other terminals' output), put files,
// diffs and commits in front of them, and take or restore checkpoints.
// The server (Rust, mcp.rs) only listens on 127.0.0.1 and needs a secret
// token; tool calls are answered here.
import type { EditorView } from "@codemirror/view";
import { errorText, ipc, type Checkpoint, type McpInfo } from "./ipc";
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
  "read_terminal shows the output of the user's other terminals (a dev server, tests, a build): check it for errors after you change code, instead of asking the user to copy them.",
  "Before a risky or wide change, call create_checkpoint with a short note; the user can then restore it from KlikVibe. list_checkpoints shows what was saved. restore_checkpoint puts files back, but only after the user agrees in KlikVibe.",
  "show_commit puts a commit (or one file in it) in front of the user; reveal_in_files shows a file or folder in the file tree.",
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

/** A checkpoint by hash or the start of one. */
async function findCheckpoint(p: ProjectState, ref: string): Promise<Checkpoint> {
  const list = await ipc.checkpointList(p.root);
  const want = ref.trim().toLowerCase();
  const found = list.filter((c) => c.hash.startsWith(want));
  if (found.length !== 1) throw new Error(found.length ? `"${ref}" matches several checkpoints; give more of the hash.` : `No checkpoint "${ref}" here; list_checkpoints shows them.`);
  return found[0];
}

const describeCheckpoint = (c: Checkpoint) => ({ hash: c.short, date: c.date, reason: c.reason, note: c.note || undefined, files: c.files });

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
    name: "read_terminal",
    description:
      "The recent output of the terminals in a project (a dev server, tests, a build), as plain text. Without `terminal`, every terminal except those running an agent. Use it to see errors after a change.",
    inputSchema: {
      type: "object",
      properties: {
        project: { type: "string", description: "Project root or name (optional)." },
        terminal: { type: "string", description: "The terminal's name (as in KlikVibe, e.g. \"npm run dev\") or number, optional." },
        lines: { type: "number", description: "Lines from the end, per terminal (default 60, at most 500)." },
      },
    },
    run: (args) => {
      const p = pickProject(args.project);
      const all = p.terminals?.() ?? [];
      if (!all.length) return "No terminals are open in this project.";
      const lines = Math.min(500, num(args.lines) ?? 60);
      const which = typeof args.terminal === "string" && args.terminal.trim() ? args.terminal.trim() : null;
      const picked = which
        ? all.filter((x) => String(x.index) === which || x.name.toLowerCase() === which.toLowerCase())
        : all.filter((x) => !x.agent);
      if (!picked.length) return `No terminal "${which ?? ""}". Terminals: ${all.map((x) => `${x.index} "${x.name}"${x.agent ? " (agent)" : ""}`).join(", ")}.`;
      return cap(picked.map((x) => `── ${x.index}: ${x.name}${x.active ? " (in front)" : ""} ──\n${x.tail(lines) || "(no output)"}`).join("\n\n"));
    },
  },
  {
    name: "create_checkpoint",
    description: "Saves the project's files as a checkpoint (apart from commits and branches) that the user can restore from KlikVibe. Call it before a risky or wide change.",
    inputSchema: {
      type: "object",
      properties: {
        project: { type: "string", description: "Project root or name (optional)." },
        note: { type: "string", description: "What comes next, e.g. \"before moving auth to middleware\"." },
      },
    },
    run: async (args) => {
      const p = pickProject(args.project);
      const note = typeof args.note === "string" ? args.note : "";
      const made = await p.takeCheckpoint("agent", note);
      if (p.checkpointsOff) throw new Error(p.checkpointsOff);
      if (!made) {
        const [latest] = await ipc.checkpointList(p.root);
        return `Nothing changed since the last checkpoint${latest ? ` (${latest.short})` : ""}; that one already holds the current files.`;
      }
      return out({ created: describeCheckpoint(made) });
    },
  },
  {
    name: "list_checkpoints",
    description: "The project's checkpoints, newest first, with why each was taken and how many files changed. With `checkpoint`, also the files that one changed.",
    inputSchema: {
      type: "object",
      properties: {
        project: { type: "string", description: "Project root or name (optional)." },
        checkpoint: { type: "string", description: "A checkpoint's hash (or its start) to list its files (optional)." },
        limit: { type: "number", description: "How many (default 20)." },
      },
    },
    run: async (args) => {
      const p = pickProject(args.project);
      if (typeof args.checkpoint === "string" && args.checkpoint.trim()) {
        const c = await findCheckpoint(p, args.checkpoint);
        const files = await ipc.gitCommitFiles(p.root, c.hash);
        return out({ ...describeCheckpoint(c), changed: files.map((f) => ({ path: f.path, kind: f.kind, added: f.added, removed: f.removed })) });
      }
      const list = await ipc.checkpointList(p.root);
      return out({ checkpoints: list.slice(0, num(args.limit) ?? 20).map(describeCheckpoint) });
    },
  },
  {
    name: "restore_checkpoint",
    description:
      "Asks the user, in KlikVibe, to put the project's files back as they were at a checkpoint (files changed since are rewritten, files added since are deleted; the current state is saved first). Returns whether they agreed.",
    inputSchema: {
      type: "object",
      properties: {
        project: { type: "string", description: "Project root or name (optional)." },
        checkpoint: { type: "string", description: "The checkpoint's hash (or its start), from list_checkpoints." },
      },
      required: ["checkpoint"],
    },
    run: async (args) => {
      const p = pickProject(args.project);
      const c = await findCheckpoint(p, str(args.checkpoint, "checkpoint"));
      showProject(p.root);
      settings.showGit = true;
      persist();
      p.gitTab = "checkpoints";
      p.loadCheckpoints();
      const when = new Date(c.date).toLocaleString();
      return (await p.restoreCheckpoint(c, when)) ? `Restored the files to checkpoint ${c.short} (${when}).` : "The user kept the files as they are.";
    },
  },
  {
    name: "show_commit",
    description: "Opens a commit in KlikVibe for the user to review: the whole commit, or with `path` just that file's changes in it.",
    inputSchema: {
      type: "object",
      properties: {
        project: { type: "string", description: "Project root or name (optional)." },
        commit: { type: "string", description: "The commit hash (at least 4 characters)." },
        path: { type: "string", description: "A file changed in that commit, relative to the project (optional)." },
      },
      required: ["commit"],
    },
    run: async (args) => {
      const p = pickProject(args.project);
      const hash = str(args.commit, "commit").toLowerCase();
      if (!/^[0-9a-f]{4,64}$/.test(hash)) throw new Error("Give the commit's hash (hex), e.g. from git log.");
      const commit = { hash, short: hash.slice(0, 7) };
      showProject(p.root);
      if (typeof args.path === "string" && args.path.trim()) {
        const rel = args.path.trim().replace(/^\.\//, "");
        const file = (await ipc.gitCommitFiles(p.root, hash)).find((f) => f.path === rel);
        if (!file) throw new Error(`${rel} didn't change in ${commit.short}.`);
        await p.openCommitFile(commit, file);
        return `Showing the changes to ${rel} in ${commit.short}.`;
      }
      await p.openCommit(commit);
      return `Showing commit ${commit.short}.`;
    },
  },
  {
    name: "reveal_in_files",
    description: "Shows a file or folder in KlikVibe's file tree (opening the folders down to it), without opening the file.",
    inputSchema: { type: "object", properties: { path: { type: "string", description: "Absolute, or relative to the project in front." } }, required: ["path"] },
    run: async (args) => {
      const { project, abs } = locate(str(args.path, "path"));
      const isDir = await ipc.listDir(abs).then(
        () => true,
        () => false,
      );
      showProject(project.root);
      await project.revealInTree(abs, isDir);
      return `Showing ${relative(project.root, abs) || "."} in the file tree.`;
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
        await ipc.mcpReply(call.id, { content: [{ type: "text", text: errorText(e) }], isError: true });
      }
    });
    mcp.error = null;
    settings.mcpEnabled = true;
  } catch (e) {
    mcp.error = errorText(e);
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
