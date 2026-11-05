<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { errorText, ipc, type Project } from "./lib/ipc";
  import { activeTab, closeTab, isDirty, openProject, project, saveTab } from "./lib/project.svelte";
  import { persist, settings } from "./lib/settings.svelte";
  import { toast, ui } from "./lib/ui.svelte";
  import { i18n } from "./lib/i18n/index.svelte";
  import Home from "./components/Home.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import FileTree from "./components/FileTree.svelte";
  import Editor from "./components/Editor.svelte";
  import GitPanel from "./components/GitPanel.svelte";
  import TerminalPanel from "./components/TerminalPanel.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Icon from "./components/Icon.svelte";

  onMount(() => {
    document.documentElement.lang = i18n.locale;
    const off = listen<Project>("open-project", (e) => openProject(e.payload));
    ipc.takeLaunchProject().then((p) => p && openProject(p));
    return () => off.then((f) => f());
  });

  // The window title follows the project and the active file.
  $effect(() => {
    const tab = activeTab();
    const name = project.current?.name;
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

  function toggle(key: "showTree" | "showGit" | "showTerminal") {
    settings[key] = !settings[key];
    persist();
  }

  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (e.ctrlKey && e.key === "`") {
      e.preventDefault();
      if (project.current) toggle("showTerminal");
    } else if (mod && !e.shiftKey && e.key.toLowerCase() === "o") {
      e.preventDefault();
      pick();
    } else if (!project.current) {
      return;
    } else if (mod && !e.shiftKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      saveTab();
    } else if (mod && !e.shiftKey && e.key.toLowerCase() === "w") {
      e.preventDefault();
      if (project.active) closeTab(project.active);
    } else if (mod && !e.shiftKey && e.key.toLowerCase() === "b") {
      e.preventDefault();
      toggle("showTree");
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "g") {
      e.preventDefault();
      toggle("showGit");
    }
  }

  function beforeUnload(e: BeforeUnloadEvent) {
    if (project.tabs.some(isDirty)) e.preventDefault();
  }

  /** Drags a side panel's edge; `dir` is +1 for the left panel, -1 for the right one. */
  function resize(e: PointerEvent, key: "treeWidth" | "gitWidth", dir: 1 | -1) {
    const startX = e.clientX;
    const start = settings[key];
    const el = e.currentTarget as HTMLElement;
    el.classList.add("active");
    const move = (ev: PointerEvent) => (settings[key] = Math.min(560, Math.max(180, start + dir * (ev.clientX - startX))));
    const up = () => {
      el.classList.remove("active");
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      persist();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
</script>

<svelte:window onkeydown={onKeydown} onbeforeunload={beforeUnload} />

<div class="app">
  {#if project.current}
    <TitleBar />
    <div class="main">
      {#if settings.showTree}
        <div class="side" style:width="{settings.treeWidth}px"><FileTree /></div>
        <div class="resizer" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "treeWidth", 1)}></div>
      {/if}
      <div class="center">
        <Editor />
        {#key project.current.root}
          <TerminalPanel />
        {/key}
      </div>
      {#if settings.showGit}
        <div class="resizer" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "gitWidth", -1)}></div>
        <div class="side right" style:width="{settings.gitWidth}px"><GitPanel /></div>
      {/if}
    </div>
  {:else}
    <Home />
  {/if}

  {#if ui.busy}
    <div class="busy"><Icon name="loader" size={13} class="spin" />{ui.busy}</div>
  {/if}
  <Toasts />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .side {
    flex: none;
    min-width: 0;
    border-right: 1px solid var(--line);
  }
  .side.right {
    border-right: none;
    border-left: 1px solid var(--line);
  }
  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
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
