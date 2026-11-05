<script lang="ts">
  import { closeProject, currentProject, showProject, workspace, type ProjectState } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { copyText, isMac, mod, openMenu, reveal, revealLabel } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import McpBadge from "./McpBadge.svelte";

  const project = $derived(currentProject());
  const git = $derived(project?.git);

  function toggle(key: "showTree" | "showGit" | "showTerminal") {
    settings[key] = !settings[key];
    persist();
  }

  /** Closes projects one by one; stops when the user keeps unsaved edits in one. */
  async function closeMany(list: ProjectState[]) {
    for (const p of list) if (!(await closeProject(p))) return;
  }

  function projectMenu(e: MouseEvent, p: ProjectState) {
    const others = workspace.projects.filter((x) => x !== p);
    openMenu(e, [
      { label: t("menu.closeProject"), run: () => closeProject(p) },
      { label: t("menu.closeOtherProjects"), disabled: !others.length, run: () => closeMany(others) },
      { label: t("menu.closeAllProjects"), run: () => closeMany([...workspace.projects]) },
      "sep",
      { label: t("menu.copyPath"), run: () => copyText(p.root) },
      { label: revealLabel(), run: () => reveal(p.root) },
    ]);
  }

  function onAux(e: MouseEvent, p: ProjectState) {
    // Middle click closes, as in browsers.
    if (e.button === 1) {
      e.preventDefault();
      closeProject(p);
    }
  }
</script>

<header class="bar" class:mac={isMac} data-tauri-drag-region>
  <button class="icon-btn" class:on={!project} onclick={() => showProject(null)} title={t("bar.home")} aria-label={t("bar.home")}><Icon name="home" size={15} /></button>
  <div class="projects" role="tablist" data-tauri-drag-region>
    {#each workspace.projects as p, i (p.root)}
      {@const on = p.root === workspace.active}
      <div class="ptab" class:on role="tab" aria-selected={on} tabindex="-1">
        <button class="ptab-main" onclick={() => showProject(p.root)} onauxclick={(e) => onAux(e, p)} oncontextmenu={(e) => projectMenu(e, p)} title={i < 9 ? `${p.root} (${mod}${i + 1})` : p.root}>
          <Icon name="project" size={13} />
          <span class="name">{p.current.name}</span>
          {#if p.git?.changes.length}<span class="dot" title={t("git.changesCount", { count: p.git.changes.length })}></span>{/if}
        </button>
        <button class="ptab-close" class:dirty={p.dirty} onclick={() => closeProject(p)} title={t("bar.closeProject")} aria-label={t("bar.closeProject")}>
          <span class="unsaved"></span><Icon name="close" size={11} />
        </button>
      </div>
    {/each}
    <button class="icon-btn add" class:on={!project} onclick={() => showProject(null)} title={`${t("bar.openAnother")} (${mod}T)`} aria-label={t("bar.openAnother")}>
      <Icon name="plus" size={14} />
    </button>
  </div>
  {#if project && git?.repo}
    <button class="branch" onclick={() => ((settings.showGit = true), (project.gitTab = "changes"), persist())} title={git.upstream ?? t("git.noUpstream")}>
      <Icon name="branch" size={12} />
      <span>{git.branch ?? t("git.detached")}</span>
      {#if git.ahead}<span class="ab ahead" title={t("git.aheadHint", { count: git.ahead })}><Icon name="arrow-up" size={11} />{git.ahead}</span>{/if}
      {#if git.behind}<span class="ab behind" title={t("git.behindHint", { count: git.behind })}><Icon name="arrow-down" size={11} />{git.behind}</span>{/if}
    </button>
  {/if}
  <div class="spacer" data-tauri-drag-region></div>
  <McpBadge />
  {#if project}
    <div class="toggles">
      <button class="icon-btn" class:on={settings.showTree} onclick={() => toggle("showTree")} title={`${t("bar.files")} (${mod}B)`} aria-label={t("bar.files")} aria-pressed={settings.showTree}>
        <Icon name="panel-left" size={15} />
      </button>
      <button class="icon-btn" class:on={settings.showTerminal} onclick={() => toggle("showTerminal")} title={t("bar.terminal") + " (Ctrl+`)"} aria-label={t("bar.terminal")} aria-pressed={settings.showTerminal}>
        <Icon name="terminal" size={15} />
      </button>
      <button class="icon-btn" class:on={settings.showGit} onclick={() => toggle("showGit")} title={`${t("bar.git")} (${mod}⇧G)`} aria-label={t("bar.git")} aria-pressed={settings.showGit}>
        <Icon name="panel-right" size={15} />
      </button>
    </div>
  {/if}
</header>

<style>
  .bar {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .bar.mac {
    padding-left: 80px;
  }
  .projects {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .projects::-webkit-scrollbar {
    display: none;
  }
  .ptab {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: 7px;
    border: 1px solid transparent;
    color: var(--muted);
    flex: none;
    max-width: 220px;
  }
  .ptab:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .ptab.on {
    background: var(--bg);
    border-color: var(--line);
    color: var(--text);
  }
  .ptab-main {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    min-width: 0;
    padding: 0 4px 0 10px;
    background: none;
    border: none;
    color: inherit;
  }
  .ptab.on .ptab-main :global(svg) {
    color: var(--accent);
  }
  .name {
    font: 500 13.5px var(--serif);
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
  }
  .ptab-close {
    width: 20px;
    height: 20px;
    margin-right: 4px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    opacity: 0;
    flex: none;
  }
  .ptab:hover .ptab-close,
  .ptab.on .ptab-close,
  .ptab-close.dirty {
    opacity: 1;
  }
  .ptab-close:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .ptab-close > * {
    grid-area: 1 / 1;
  }
  .unsaved {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text);
    display: none;
  }
  .ptab-close.dirty:not(:hover) .unsaved {
    display: block;
  }
  .ptab-close.dirty:not(:hover) :global(svg) {
    visibility: hidden;
  }
  .add {
    flex: none;
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--bg);
    font: 11px var(--mono);
    color: var(--text);
    flex: none;
  }
  .branch:hover {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .branch :global(svg) {
    color: var(--muted);
  }
  .ab {
    display: flex;
    align-items: center;
    gap: 1px;
  }
  .ab.ahead,
  .ab.ahead :global(svg) {
    color: var(--accent);
  }
  .ab.behind,
  .ab.behind :global(svg) {
    color: var(--accent-2);
  }
  .spacer {
    flex: 1;
    align-self: stretch;
    min-width: 24px;
  }
  .toggles {
    display: flex;
    gap: 2px;
    flex: none;
  }
</style>
