<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { errorText, ipc, type Project } from "./lib/ipc";
  import { confirmQuit, currentProject, cycleProject, isDirty, openProject, showProject, workspace } from "./lib/project.svelte";
  import { persist, settings, zoom } from "./lib/settings.svelte";
  import { isMac, toast, ui } from "./lib/ui.svelte";
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
  import { seen } from "./lib/attention.svelte";
  import { restoreSession, saveSession } from "./lib/session.svelte";
  import { treeKey } from "./lib/grid";

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

  async function pick() {
    try {
      const p = await ipc.pickProject();
      if (p) await openProject(p);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  /** Shows the file tree and puts the cursor in its search (names or contents). */
  function openSearch(mode: "names" | "text") {
    settings.showCode = true;
    settings[treeKey(settings)] = true;
    persist();
    window.dispatchEvent(new CustomEvent("klikvibe:search", { detail: mode }));
  }

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
    } else if (inTerminal && !e.shiftKey) {
      return;
    } else if (mod && !e.shiftKey && key === "o") {
      e.preventDefault();
      pick();
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

  function beforeUnload(e: BeforeUnloadEvent) {
    if (workspace.projects.some((p) => p.dirty)) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} onbeforeunload={beforeUnload} onfocus={() => workspace.active && seen(workspace.active)} />

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
