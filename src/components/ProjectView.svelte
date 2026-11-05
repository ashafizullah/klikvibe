<script lang="ts">
  // One project's workspace: files, editor, terminals and Git. Every open
  // project keeps one of these mounted, so its terminals keep running while
  // another project is in front.
  import { provideProject, type ProjectState } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import FileTree from "./FileTree.svelte";
  import Editor from "./Editor.svelte";
  import GitPanel from "./GitPanel.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  let { project, shown }: { project: ProjectState; shown: boolean } = $props();
  // svelte-ignore state_referenced_locally
  provideProject(project);

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

<div class="main" class:hidden={!shown}>
  {#if settings.showTree}
    <div class="side" style:width="{settings.treeWidth}px"><FileTree /></div>
    <div class="resizer" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "treeWidth", 1)}></div>
  {/if}
  <div class="center">
    <Editor />
    <TerminalPanel {shown} />
  </div>
  {#if settings.showGit}
    <div class="resizer" role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "gitWidth", -1)}></div>
    <div class="side right" style:width="{settings.gitWidth}px"><GitPanel /></div>
  {/if}
</div>

<style>
  .main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .main.hidden {
    display: none;
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
</style>
