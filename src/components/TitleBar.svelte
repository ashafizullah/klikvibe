<script lang="ts">
  import { closeProject, project } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { isMac, mod } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  const git = $derived(project.git);

  function toggle(key: "showTree" | "showGit" | "showTerminal") {
    settings[key] = !settings[key];
    persist();
  }
</script>

<header class="bar" class:mac={isMac} data-tauri-drag-region>
  <button class="icon-btn" onclick={() => closeProject()} title={t("bar.home")} aria-label={t("bar.home")}><Icon name="home" size={15} /></button>
  <div class="project" data-tauri-drag-region>
    <img src="/icon.png" alt="" width="18" height="18" draggable="false" />
    <span class="name" data-tauri-drag-region>{project.current?.name}</span>
    {#if git?.repo}
      <button class="branch" onclick={() => ((settings.showGit = true), (project.gitTab = "changes"), persist())} title={git.upstream ?? t("git.noUpstream")}>
        <Icon name="branch" size={12} />
        <span>{git.branch ?? t("git.detached")}</span>
        {#if git.ahead}<span class="ab ahead" title={t("git.aheadHint", { count: git.ahead })}><Icon name="arrow-up" size={11} />{git.ahead}</span>{/if}
        {#if git.behind}<span class="ab behind" title={t("git.behindHint", { count: git.behind })}><Icon name="arrow-down" size={11} />{git.behind}</span>{/if}
        {#if git.changes.length}<span class="dot" title={t("git.changesCount", { count: git.changes.length })}></span>{/if}
      </button>
    {/if}
  </div>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="toggles">
    <button class="icon-btn" class:on={settings.showTree} onclick={() => toggle("showTree")} title={`${t("bar.files")} (${mod}B)`} aria-label={t("bar.files")} aria-pressed={settings.showTree}>
      <Icon name="panel-left" size={15} />
    </button>
    <button class="icon-btn" class:on={settings.showTerminal} onclick={() => toggle("showTerminal")} title={t("bar.terminal") + " (Ctrl+`)"} aria-label={t("bar.terminal")} aria-pressed={settings.showTerminal}>
      <Icon name="panel-bottom" size={15} />
    </button>
    <button class="icon-btn" class:on={settings.showGit} onclick={() => toggle("showGit")} title={`${t("bar.git")} (${mod}⇧G)`} aria-label={t("bar.git")} aria-pressed={settings.showGit}>
      <Icon name="panel-right" size={15} />
    </button>
  </div>
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
  .project {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .project img {
    border-radius: 4px;
    flex: none;
  }
  .name {
    font: 500 15px var(--serif);
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  .toggles {
    display: flex;
    gap: 2px;
  }
</style>
