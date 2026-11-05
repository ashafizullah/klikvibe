<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { errorText, ipc, type Project } from "./lib/ipc";
  import { currentProject, cycleProject, isDirty, openProject, showProject, workspace } from "./lib/project.svelte";
  import { settings } from "./lib/settings.svelte";
  import { toast, ui } from "./lib/ui.svelte";
  import { i18n } from "./lib/i18n/index.svelte";
  import { initUpdates } from "./lib/updates.svelte";
  import Home from "./components/Home.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ProjectView from "./components/ProjectView.svelte";
  import Toasts from "./components/Toasts.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import Tooltip from "./components/Tooltip.svelte";
  import McpDialog from "./components/McpDialog.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import AlertDialog from "./components/AlertDialog.svelte";
  import { startMcp } from "./lib/mcp.svelte";
  import { togglePanel } from "./lib/layout";
  import Icon from "./components/Icon.svelte";

  const project = $derived(currentProject());

  onMount(() => {
    document.documentElement.lang = i18n.locale;
    const off = listen<Project>("open-project", (e) => openProject(e.payload));
    ipc.takeLaunchProject().then((p) => p && openProject(p));
    initUpdates();
    if (settings.mcpEnabled) startMcp();
    return () => off.then((f) => f());
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

  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
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
    }
  }

  function beforeUnload(e: BeforeUnloadEvent) {
    if (workspace.projects.some((p) => p.dirty)) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} onbeforeunload={beforeUnload} />

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

  {#if ui.busy}
    <div class="busy"><Icon name="loader" size={13} class="spin" />{ui.busy}</div>
  {/if}
  {#if ui.dialog === "mcp"}
    <McpDialog />
  {:else if ui.dialog === "about"}
    <AboutDialog />
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
  .busy {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    background: var(--panel-2);
    border: 1px solid var(--line);
    color: var(--text);
    font-size: 12px;
    padding: 7px 14px;
    border-radius: 999px;
    display: flex;
    gap: 8px;
    align-items: center;
    z-index: 90;
    box-shadow: var(--shadow);
  }
</style>
