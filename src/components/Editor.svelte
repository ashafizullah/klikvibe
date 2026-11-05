<script lang="ts">
  import { isDirty, useProject, type Tab } from "../lib/project.svelte";
  import { languageName } from "../lib/languages";
  import { relative } from "../lib/paths";
  import { persist, settings } from "../lib/settings.svelte";
  import { copyText, mod, openMenu, reveal, revealLabel, toast } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import CodeView from "./CodeView.svelte";
  import DiffView from "./DiffView.svelte";
  import MediaView from "./MediaView.svelte";
  import { errorText, ipc } from "../lib/ipc";
  import { hasSource } from "../lib/media";

  const project = useProject();
  const tab = $derived(project.activeTab());
  const rel = $derived(tab ? relative(project.root, tab.path) : null);
  const ICON = { file: "code", diff: "diff", commit: "commit" } as const;

  function onAux(e: MouseEvent, id: string) {
    // Middle click closes, as in browsers.
    if (e.button === 1) {
      e.preventDefault();
      project.closeTab(id);
    }
  }

  const tabIcon = (tb: Tab) => (tb.kind === "file" && tb.media ? tb.media : ICON[tb.kind]);

  function tabMenu(e: MouseEvent, tb: Tab) {
    const i = project.tabs.indexOf(tb);
    const ids = (list: Tab[]) => list.map((x) => x.id);
    const others = project.tabs.filter((x) => x.id !== tb.id);
    const right = project.tabs.slice(i + 1);
    const saved = project.tabs.filter((x) => !isDirty(x));
    const where = tb.kind === "commit" ? [{ label: t("menu.copyHash"), run: () => copyText(tb.id.slice("commit:".length)) }] : [
      { label: t("menu.copyPath"), run: () => copyText(tb.path) },
      { label: t("menu.copyRelative"), run: () => copyText(relative(project.root, tb.path) ?? tb.path) },
      "sep" as const,
      { label: revealLabel(), run: () => reveal(tb.path) },
    ];
    openMenu(e, [
      { label: t("menu.close"), hint: `${mod}W`, run: () => project.closeTab(tb.id) },
      { label: t("menu.closeOthers"), disabled: !others.length, run: () => project.closeTabs(ids(others), tb.id) },
      { label: t("menu.closeRight"), disabled: !right.length, run: () => project.closeTabs(ids(right), tb.id) },
      { label: t("menu.closeSaved"), disabled: !saved.length, run: () => project.closeTabs(ids(saved)) },
      { label: t("menu.closeAll"), run: () => project.closeTabs(ids(project.tabs)) },
      "sep",
      ...where,
    ]);
  }

  function openExternally(path: string) {
    ipc.openExternally(path).catch((e) => toast(errorText(e), "error"));
  }

  function toggleWrap() {
    settings.wrap = !settings.wrap;
    persist();
  }
</script>

<section class="editor">
  {#if project.tabs.length}
    <div class="tabs" role="tablist">
      {#each project.tabs as tb (tb.id)}
        <div class="tab" class:on={tb.id === project.active} role="tab" aria-selected={tb.id === project.active} tabindex="-1">
          <button class="tab-main" onclick={() => (project.active = tb.id)} onauxclick={(e) => onAux(e, tb.id)} oncontextmenu={(e) => tabMenu(e, tb)} title={tb.path}>
            <Icon name={tabIcon(tb)} size={13} />
            <span class="tab-name">{tb.title}</span>
          </button>
          <button class="tab-close" class:dirty={isDirty(tb)} onclick={() => project.closeTab(tb.id)} aria-label={t("tabs.close")} title={t("tabs.close")}>
            <span class="dot"></span><Icon name="close" size={12} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if tab}
    {#if tab.kind === "file" && tab.changedOnDisk}
      <div class="banner">
        <Icon name="alert" size={14} />
        <span>{t("tabs.changedOnDisk")}</span>
        <button class="btn small" onclick={() => project.reloadTab(tab)}>{t("tabs.reload")}</button>
        <button class="btn small ghost" onclick={() => (tab.changedOnDisk = false)}>{t("tabs.keepMine")}</button>
      </div>
    {/if}
    {#if tab.kind === "file"}
      {#if tab.media}
        {#key tab.id}
          <MediaView {tab} onopen={() => openExternally(tab.path)} />
        {/key}
      {:else if tab.loading}
        <p class="empty-note">{t("common.loading")}</p>
      {:else if tab.binary || tab.tooLarge}
        <div class="notice">
          <Icon name="file" size={28} stroke={1.5} />
          <p>{tab.tooLarge ? t("tabs.tooLarge") : t("tabs.binary")}</p>
        </div>
      {:else}
        {#key tab.id}
          <CodeView {tab} />
        {/key}
      {/if}
      <footer class="status">
        <span class="mono path">{rel ?? tab.path}</span>
        <span class="spacer"></span>
        {#if isDirty(tab)}
          <button class="btn ghost tiny" onclick={() => project.saveTab(tab)}><Icon name="save" size={12} />{t("tabs.save")} <span class="kbd">{mod}S</span></button>
        {/if}
        {#if tab.media}
          {#if hasSource(tab.path)}
            <button class="btn ghost tiny" onclick={() => project.showSource(tab)}><Icon name="code" size={12} />{t("media.source")}</button>
          {/if}
          <button class="btn ghost tiny" onclick={() => openExternally(tab.path)}><Icon name="external" size={12} />{t("media.openExternally")}</button>
          <span>{t(`media.${tab.media}`)}</span>
        {:else}
          <button class="btn ghost tiny" class:on={settings.wrap} onclick={toggleWrap} title={t("tabs.wrap")}><Icon name="wrap" size={12} /></button>
          <span>{languageName(tab.title)}</span>
        {/if}
      </footer>
    {:else}
      {#key tab.id}
        <DiffView {tab} />
      {/key}
    {/if}
  {:else}
    <div class="welcome grain-free">
      <div class="welcome-card rise">
        <div class="hero-icon"><Icon name="sparkles" size={22} /></div>
        <h2>{t("editor.welcomeTitle")}</h2>
        <p>{t("editor.welcomeText")}</p>
        <ul>
          <li><Icon name="terminal" size={13} />{t("editor.tipTerminal")}</li>
          <li><Icon name="diff" size={13} />{t("editor.tipGit")}</li>
          <li><Icon name="code" size={13} />{t("editor.tipTree")}</li>
        </ul>
      </div>
    </div>
  {/if}
</section>

<style>
  .editor {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
  }
  .tabs {
    height: 34px;
    flex: none;
    display: flex;
    overflow-x: auto;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    scrollbar-width: none;
  }
  .tab {
    display: flex;
    align-items: center;
    flex: none;
    max-width: 220px;
    border-right: 1px solid var(--line);
    color: var(--muted);
  }
  .tab.on {
    background: var(--canvas);
    color: var(--text);
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .tab-main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 100%;
    padding: 0 4px 0 12px;
    border: none;
    background: none;
    font-size: 12px;
  }
  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tab-close {
    width: 22px;
    height: 22px;
    margin-right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    padding: 0;
  }
  .tab-close:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .tab-close .dot {
    display: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }
  .tab-close.dirty .dot {
    display: block;
  }
  .tab-close.dirty :global(svg) {
    display: none;
  }
  .tab-close.dirty:hover .dot {
    display: none;
  }
  .tab-close.dirty:hover :global(svg) {
    display: block;
  }
  .banner {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    background: color-mix(in srgb, var(--accent) 12%, var(--panel));
    border-bottom: 1px solid var(--line);
  }
  .banner :global(svg) {
    color: var(--accent);
  }
  .banner span {
    flex: 1;
  }
  .small {
    height: 24px;
  }
  .notice {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .status {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
    font-size: 11px;
    color: var(--muted);
  }
  .path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .tiny {
    height: 20px;
    padding: 0 6px;
    font-size: 11px;
    gap: 4px;
  }
  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .welcome-card {
    max-width: 460px;
    padding: 24px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: linear-gradient(
      to bottom right,
      color-mix(in srgb, var(--accent) 14%, transparent),
      var(--panel) 50%,
      color-mix(in srgb, var(--accent-2) 9%, transparent)
    );
  }
  .hero-icon {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    margin-bottom: 12px;
  }
  .welcome h2 {
    margin: 0;
    font: 500 20px var(--serif);
    letter-spacing: -0.015em;
  }
  .welcome p {
    margin: 4px 0 14px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .welcome ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
  }
  .welcome li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .welcome li :global(svg) {
    color: var(--accent);
    flex: none;
  }
</style>
