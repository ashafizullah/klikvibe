// Everything the command palette (⌘⇧P / Ctrl+Shift+P) can do, for the project in front.
import { errorText, ipc, type RecentProject } from "./ipc";
import { closeProject, currentProject, newTask, openProject, showProject, workspace, type ProjectState } from "./project.svelte";
import { persist, settings, zoom } from "./settings.svelte";
import { isLastPanel, setLayout, togglePanel, type PanelKey } from "./layout";
import { treeKey } from "./grid";
import { promptList } from "./prompts.svelte";
import { checkForUpdates } from "./updates.svelte";
import { i18n, setLocale, t } from "./i18n/index.svelte";
import { setTheme } from "./theme.svelte";
import { isMac, mod, toast, ui } from "./ui.svelte";

export interface Command {
  /** Stable, for "used last" at the top of the list. */
  id: string;
  group: string;
  label: string;
  icon: string;
  /** Its shortcut, shown beside it. */
  keys?: string;
  run: () => unknown;
}

/** Asks for a folder and opens it as a project. */
export async function pickFolder() {
  try {
    const p = await ipc.pickProject();
    if (p) await openProject(p);
  } catch (e) {
    toast(errorText(e), "error");
  }
}

/** Shows the file tree and puts the cursor in its search (names or contents). */
export function openSearch(mode: "names" | "text") {
  settings.showCode = true;
  settings[treeKey(settings)] = true;
  persist();
  window.dispatchEvent(new CustomEvent("klikvibe:search", { detail: mode }));
}

const shift = (key: string) => (isMac ? `⌘⇧${key}` : `Ctrl+Shift+${key}`);

function showTerminal() {
  if (settings.showTerminal) return;
  settings.showTerminal = true;
  persist();
}

function showGit(p: ProjectState, tab: ProjectState["gitTab"]) {
  settings.showGit = true;
  p.gitTab = tab;
  persist();
}

/** The commands, for what is open right now; `recent`: projects to offer that aren't open. */
export function commands(recent: RecentProject[]): Command[] {
  const p = currentProject();
  const list: Command[] = [];
  const add = (c: Command) => list.push(c);
  const g = {
    file: t("palette.gFile"),
    terminal: t("terminal.title"),
    agent: t("palette.gAgent"),
    git: t("bar.git"),
    view: t("palette.gView"),
    project: t("palette.gProject"),
    app: "KlikVibe",
  };

  if (p) {
    add({ id: "file.names", group: g.file, label: t("search.namesHint"), icon: "search", keys: `${mod}P`, run: () => openSearch("names") });
    add({ id: "file.text", group: g.file, label: t("search.textHint"), icon: "search", keys: shift("F"), run: () => openSearch("text") });
    const tab = p.activeTab();
    if (tab) {
      add({ id: "file.save", group: g.file, label: t("palette.save"), icon: "save", keys: `${mod}S`, run: () => p.saveTab() });
      add({ id: "file.close", group: g.file, label: t("palette.closeTab"), icon: "close", keys: `${mod}W`, run: () => p.closeTab(tab.id) });
    }

    add({
      id: "terminal.new",
      group: g.terminal,
      label: t("terminal.new"),
      icon: "plus",
      run: () => {
        showTerminal();
        p.newTerminal?.(null).catch(() => {});
      },
    });
    const split = p.splitTerminal;
    if (split)
      add({
        id: "terminal.split",
        group: g.terminal,
        label: split.on() ? t("terminal.unsplit") : t("terminal.split"),
        icon: "columns-2",
        run: () => {
          showTerminal();
          split.toggle();
        },
      });
    if (p.findInTerminal) add({ id: "terminal.find", group: g.terminal, label: t("terminal.find"), icon: "search", keys: isMac ? "⌘F" : "Ctrl+Shift+F", run: () => p.findInTerminal?.() });

    for (const text of promptList().filter((x) => x.trim()))
      add({ id: `agent.prompt:${text}`, group: g.agent, label: t("palette.ask", { prompt: text.replace(/\s+/g, " ") }), icon: "message-square-text", run: () => p.askAgent?.(text) });
    add({ id: "agent.prompts", group: g.agent, label: t("prompts.edit"), icon: "message-square-text", run: () => (ui.dialog = "prompts") });

    const git = p.git;
    if (git?.repo) {
      add({ id: "git.changes", group: g.git, label: t("git.changes"), icon: "diff", keys: shift("G"), run: () => showGit(p, "changes") });
      add({ id: "git.history", group: g.git, label: t("git.history"), icon: "history", run: () => showGit(p, "history") });
      add({ id: "git.checkpoints", group: g.git, label: t("git.checkpoints"), icon: "history", run: () => showGit(p, "checkpoints") });
      if (git.changes.some((c) => c.unstaged)) add({ id: "git.stageAll", group: g.git, label: t("git.stageAll"), icon: "plus", run: () => p.stage(git.changes.filter((c) => c.unstaged).map((c) => c.path)) });
      if (git.changes.some((c) => c.staged)) add({ id: "git.unstageAll", group: g.git, label: t("git.unstageAll"), icon: "minus", run: () => p.unstage(git.changes.filter((c) => c.staged).map((c) => c.path)) });
      if (git.upstream) add({ id: "git.pull", group: g.git, label: t("git.pull"), icon: "pull", run: p.pull });
      add({ id: "git.push", group: g.git, label: git.upstream ? t("git.push") : t("git.publish"), icon: "push", run: p.push });
      add({ id: "git.refresh", group: g.git, label: t("git.refresh"), icon: "refresh", run: () => p.refreshGit(true) });
      if (!p.checkpointsOff) add({ id: "git.checkpoint", group: g.git, label: t("checkpoints.now"), icon: "save", run: () => p.takeCheckpoint("manual") });
      add({ id: "git.task", group: g.git, label: t("tasks.new"), icon: "branch-plus", run: () => newTask(p) });
    } else if (git) add({ id: "git.init", group: g.git, label: t("git.init"), icon: "branch", run: p.initRepo });

    const panels: [PanelKey, string, string, string?][] = [
      ["showTree", t("bar.files"), "panel-left", `${mod}B`],
      ["showCode", t("bar.code"), "code", shift("E")],
      ["showGit", t("bar.git"), "panel-right", shift("G")],
      ["showTerminal", t("bar.terminal"), "terminal", "Ctrl+`"],
    ];
    for (const [key, panel, icon, keys] of panels)
      if (!isLastPanel(key)) add({ id: `view.${key}`, group: g.view, label: t("palette.toggle", { panel }), icon, keys, run: () => togglePanel(key) });
    if (settings.layout === "classic") add({ id: "view.columns", group: g.view, label: t("bar.layoutColumns"), icon: "columns", run: () => setLayout("columns") });
    else add({ id: "view.classic", group: g.view, label: t("bar.layoutClassic"), icon: "rows", run: () => setLayout("classic") });
  }
  add({ id: "view.zoomIn", group: g.view, label: t("palette.zoomIn"), icon: "plus", keys: `${mod}=`, run: () => zoom(1) });
  add({ id: "view.zoomOut", group: g.view, label: t("palette.zoomOut"), icon: "minus", keys: `${mod}-`, run: () => zoom(-1) });
  add({ id: "view.zoomReset", group: g.view, label: t("palette.zoomReset"), icon: "rotate-ccw", keys: `${mod}0`, run: () => zoom(0) });

  add({ id: "project.open", group: g.project, label: t("home.open"), icon: "open", keys: `${mod}O`, run: pickFolder });
  add({ id: "project.home", group: g.project, label: t("bar.home"), icon: "home", keys: `${mod}T`, run: () => showProject(null) });
  workspace.projects.forEach((x, i) => {
    if (x === p) return;
    add({ id: `project.go:${x.root}`, group: g.project, label: t("palette.goTo", { name: x.current.name }), icon: "project", keys: i < 9 ? `${mod}${i + 1}` : undefined, run: () => showProject(x.root) });
  });
  for (const r of recent)
    if (r.exists && !workspace.projects.some((x) => x.root === r.root))
      add({
        id: `project.recent:${r.root}`,
        group: g.project,
        label: t("palette.openRecent", { name: r.name }),
        icon: "folder",
        run: () => ipc.openRecent(r.root).then(openProject, (e) => toast(errorText(e), "error")),
      });
  if (p) add({ id: "project.close", group: g.project, label: t("bar.closeProject"), icon: "close", run: () => closeProject(p) });

  add({
    id: "app.notify",
    group: g.app,
    label: settings.notifyAgents ? t("palette.notifyOff") : t("palette.notifyOn"),
    icon: settings.notifyAgents ? "bell-off" : "bell",
    run: () => ((settings.notifyAgents = !settings.notifyAgents), persist()),
  });
  for (const [choice, icon] of [["dark", "moon"], ["light", "sun"], ["system", "monitor"]] as const)
    if (settings.theme !== choice) add({ id: `app.theme.${choice}`, group: g.app, label: `${t("theme.label")}: ${t(`theme.${choice}`)}`, icon, run: () => setTheme(choice) });
  add({ id: "app.language", group: g.app, label: t("palette.language"), icon: "globe", run: () => setLocale(i18n.locale === "en" ? "id" : "en") });
  add({ id: "app.mcp", group: g.app, label: t("mcp.title"), icon: "plug", run: () => (ui.dialog = "mcp") });
  add({ id: "app.remote", group: g.app, label: t("remote.title"), icon: "phone", run: () => (ui.dialog = "remote") });
  add({ id: "app.updates", group: g.app, label: t("update.check"), icon: "refresh", run: checkForUpdates });
  add({ id: "app.about", group: g.app, label: t("about.title"), icon: "info", run: () => (ui.dialog = "about") });
  return list;
}
