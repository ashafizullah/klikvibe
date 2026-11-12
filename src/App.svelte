<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { ipc, type Project } from "./lib/ipc";
  import { confirmQuit, currentProject, cycleProject, isDirty, openProject, showProject, workspace } from "./lib/project.svelte";
  import { settings, zoom } from "./lib/settings.svelte";
  import { isMac, ui } from "./lib/ui.svelte";
  import { i18n } from "./lib/i18n/index.svelte";
  import { initUpdates } from "./lib/updates.svelte";
  import Home from "./components/Home.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ProjectView from "./components/ProjectView.svelte";
  import Toasts from "./components/Toasts.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import Tooltip from "./components/Tooltip.svelte";
  import McpDialog from "./components/McpDialog.svelte";
  import RemoteDialog from "./components/RemoteDialog.svelte";
  import { initRemote } from "./lib/remote.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import PromptsDialog from "./components/PromptsDialog.svelte";
  import AlertDialog from "./components/AlertDialog.svelte";
  import { startMcp } from "./lib/mcp.svelte";
  import { togglePanel } from "./lib/layout";
  import { openNotified, seen } from "./lib/attention.svelte";
  import { restoreSession, saveSession } from "./lib/session.svelte";
  import { openSearch, pickFolder } from "./lib/commands.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";

  const project = $derived(currentProject());

  onMount(() => {
    document.documentElement.lang = i18n.locale;
    const off = listen<Project>("open-project", (e) => openProject(e.payload));
    // Quit (⌘Q) or the close button with unsaved edits: ask, then quit.
    const offQuit = listen("quit-asked", async () => {
      if (await confirmQuit()) ipc.quitApp().catch(() => {});
    });
    // The last session first, then a folder from the launch arguments in front of it.
    restoreSession()
      .then(() => ipc.takeLaunchProject())
      .then((p) => p && openProject(p));
    initUpdates();
    initRemote();
    if (settings.mcpEnabled) startMcp();
    return () => {
      off.then((f) => f());
      offQuit.then((f) => f());
    };
  });

  // The app asks before quitting while any project has unsaved edits.
  $effect(() => {
    const unsaved = workspace.projects.some((p) => p.dirty);
    ipc.setUnsaved(unsaved).catch(() => {});
  });

  // Remember the open projects and file tabs for the next launch.
  $effect(saveSession);

  // Looking at a project clears its "an agent is waiting" dot.
  $effect(() => {
    const root = workspace.active;
    if (root && document.hasFocus()) seen(root);
  });

  // The window title follows the project in front and its active file.
  $effect(() => {
    const tab = project?.activeTab();
    const name = project?.current.name;
    const title = name ? `${tab ? `${isDirty(tab) ? "• " : ""}${tab.title} — ` : ""}${name} — KlikVibe` : "KlikVibe";
    getCurrentWindow().setTitle(title).catch(() => {});
  });

  function onKeydown(e: KeyboardEvent) {
    // ⌘ on macOS, Ctrl elsewhere: on a Mac, Ctrl+W or Ctrl+B belong to the shell.
    const mod = isMac ? e.metaKey : e.ctrlKey;
    // Elsewhere Ctrl+letter is the shell's too while the terminal has the keyboard (Ctrl+W, Ctrl+B, Ctrl+P…).
    const inTerminal = !isMac && !!(e.target as Element | null)?.closest?.(".xterm");
    const key = e.key.toLowerCase();
    if (e.ctrlKey && e.key === "`") {
      e.preventDefault();
      if (project) togglePanel("showTerminal");
    } else if (e.ctrlKey && e.key === "Tab") {
      // Next / previous project tab.
      e.preventDefault();
      cycleProject(e.shiftKey ? -1 : 1);
    } else if (mod && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key)) {
      const p = workspace.projects[Number(e.key) - 1];
      if (p) {
        e.preventDefault();
        showProject(p.root);
      }
    } else if (mod && !e.altKey && (key === "=" || key === "+" || key === "-" || key === "0")) {
      // Text size, in the terminal too.
      e.preventDefault();
      zoom(key === "0" ? 0 : key === "-" ? -1 : 1);
    } else if (mod && e.shiftKey && !e.altKey && key === "p") {
      // The command palette, from anywhere (the terminal too); again to close it.
      e.preventDefault();
      if (!ui.alert) ui.dialog = ui.dialog === "palette" ? null : "palette";
    } else if (inTerminal && !e.shiftKey) {
      return;
    } else if (mod && !e.shiftKey && key === "o") {
      e.preventDefault();
      pickFolder();
    } else if (mod && !e.shiftKey && key === "t") {
      e.preventDefault();
      showProject(null);
    } else if (!project) {
      return;
    } else if (mod && !e.shiftKey && key === "s") {
      e.preventDefault();
      project.saveTab();
    } else if (mod && !e.shiftKey && key === "w") {
      e.preventDefault();
      if (project.active) project.closeTab(project.active);
    } else if (mod && !e.shiftKey && key === "b") {
      e.preventDefault();
      togglePanel("showTree");
    } else if (mod && e.shiftKey && key === "g") {
      e.preventDefault();
      togglePanel("showGit");
    } else if (mod && e.shiftKey && key === "e") {
      e.preventDefault();
      togglePanel("showCode");
    } else if (mod && !e.shiftKey && key === "p") {
      e.preventDefault();
      openSearch("names");
    } else if (mod && e.shiftKey && key === "f") {
      e.preventDefault();
      openSearch("text");
    }
  }

  /** Back in front: the terminal of a notification just clicked, and the project's dot goes. */
  function onFocus() {
    openNotified();
    if (workspace.active) seen(workspace.active);
  }

  function beforeUnload(e: BeforeUnloadEvent) {
    if (workspace.projects.some((p) => p.dirty)) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} onbeforeunload={beforeUnload} onfocus={onFocus} />

<div class="app">
  {#if workspace.projects.length}
    <TitleBar />
  {/if}
  {#each workspace.projects as p (p.root)}
    <ProjectView project={p} shown={p.root === workspace.active} />
  {/each}
  {#if !project}
    <Home />
  {/if}

  {#if ui.dialog === "mcp"}
    <McpDialog />
  {:else if ui.dialog === "about"}
    <AboutDialog />
  {:else if ui.dialog === "prompts"}
    <PromptsDialog />
  {:else if ui.dialog === "remote"}
    <RemoteDialog />
  {:else if ui.dialog === "palette"}
    <CommandPalette />
  {/if}
  <AlertDialog />
  <Toasts />
  <ContextMenu />
  <Tooltip />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
</style>
