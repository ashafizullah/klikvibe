<script lang="ts">
  // One project's workspace: files, editor, terminals and Git. Every open
  // project keeps one of these mounted, so its terminals keep running while
  // another project is in front. The panels sit in one CSS grid (grid.ts),
  // so changing the layout or hiding a panel moves it without remounting.
  import { provideProject, type ProjectState } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { grid } from "../lib/grid";
  import FileTree from "./FileTree.svelte";
  import Editor from "./Editor.svelte";
  import GitPanel from "./GitPanel.svelte";
  import TerminalPanel from "./TerminalPanel.svelte";

  let { project, shown }: { project: ProjectState; shown: boolean } = $props();
  // svelte-ignore state_referenced_locally
  provideProject(project);

  const g = $derived(grid(settings));
  const columns = $derived(settings.layout === "columns");

  /** Drags a column's edge; `dir` is +1 for a column left of the edge, -1 for one right of it. */
  function resize(e: PointerEvent, key: "treeWidth" | "gitWidth" | "terminalWidth", dir: 1 | -1) {
    const startX = e.clientX;
    const start = settings[key];
    const max = key === "terminalWidth" ? window.innerWidth * 0.7 : 560;
    const el = e.currentTarget as HTMLElement;
    el.classList.add("active");
    const move = (ev: PointerEvent) => (settings[key] = Math.round(Math.min(max, Math.max(180, start + dir * (ev.clientX - startX)))));
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

<div class="main" class:hidden={!shown} class:columns style:grid-template-columns={g.columns} style:grid-template-rows={g.rows} style:grid-template-areas={g.areas}>
  <div class="area tree" class:off={!g.placed.has("tree")}><FileTree /></div>
  <div class="resizer" style:grid-area="rztree" class:off={!g.placed.has("rztree")} role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "treeWidth", 1)}></div>
  <div class="area editor" class:off={!g.placed.has("editor")}><Editor /></div>
  <div class="area term" class:off={!g.placed.has("term")}><TerminalPanel {shown} fill={g.terminalFills} /></div>
  <div class="resizer" style:grid-area="rzterm" class:off={!g.placed.has("rzterm")} role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "terminalWidth", 1)}></div>
  <div class="resizer" style:grid-area="rzgit" class:off={!g.placed.has("rzgit")} role="separator" aria-orientation="vertical" onpointerdown={(e) => resize(e, "gitWidth", -1)}></div>
  <div class="area git" class:off={!g.placed.has("git")}><GitPanel /></div>
</div>

<style>
  .main {
    flex: 1;
    display: grid;
    min-height: 0;
  }
  .main.hidden,
  .off {
    display: none !important;
  }
  .area {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .area > :global(*) {
    flex: 1;
    min-height: 0;
  }
  .tree {
    grid-area: tree;
    border-right: 1px solid var(--line);
  }
  .editor {
    grid-area: editor;
  }
  .term {
    grid-area: term;
  }
  /* The collapsed strip and a sized terminal keep their own height. */
  .term > :global(*) {
    flex: none;
  }
  .columns .term {
    border-right: 1px solid var(--line);
  }
  .git {
    grid-area: git;
    border-left: 1px solid var(--line);
  }
  .resizer {
    margin: 0 -2px;
  }
</style>
