<script lang="ts">
  import { isDirty, useProject, type Tab } from "../lib/project.svelte";
  import { languageName } from "../lib/languages";
  import { relative } from "../lib/paths";
  import { persist, settings } from "../lib/settings.svelte";
  import { copyText, mod, openMenu, openMenuBelow, reveal, revealLabel, toast, type MenuItem } from "../lib/ui.svelte";
  import { join } from "../lib/paths";
  import { fileUrl, forgeName } from "../lib/forge";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import CodeView from "./CodeView.svelte";
  import DiffView from "./DiffView.svelte";
  import MediaView from "./MediaView.svelte";
  import PreviewView from "./PreviewView.svelte";
  import { errorText, ipc } from "../lib/ipc";
  import { hasSource } from "../lib/media";
  import { fileIcon } from "../lib/fileIcons";
  import { moveItem, reorder } from "../lib/reorder";

  const project = useProject();
  const tab = $derived(project.activeTab());
  const rel = $derived(tab ? relative(project.root, tab.path) : null);
  const ICON = { file: "code", diff: "diff", commit: "commit", commitFile: "diff", preview: "globe" } as const;

  function onAux(e: MouseEvent, id: string) {
    // Middle click closes, as in browsers.
    if (e.button === 1) {
      e.preventDefault();
      project.closeTab(id);
    }
  }

  /** The breadcrumb: the project, then each folder down to the file (absolute paths). */
  const crumbs = $derived.by(() => {
    if (!tab || tab.kind === "commit" || tab.kind === "preview" || !rel) return [];
    const parts = rel.split("/");
    return parts.map((name, i) => ({ name, path: join(project.root, parts.slice(0, i + 1).join("/")), last: i === parts.length - 1 }));
  });

  /** A folder's files and folders, to open one (or show a folder in the tree). */
  async function folderMenu(el: Element, dir: string) {
    if (!project.tree.has(dir)) await project.loadDir(dir);
    const entries = project.tree.get(dir) ?? [];
    const items: MenuItem[] = entries.map((e) => {
      const fi = fileIcon(e.name);
      return e.dir
        ? { label: e.name, icon: "folder", color: "var(--accent)", run: () => project.revealInTree(e.path, true) }
        : { label: e.name, icon: fi.icon, color: fi.color, run: () => project.openFile(e.path) };
    });
    if (!items.length) items.push({ label: t("tree.empty"), disabled: true, run: () => {} });
    items.push("sep", { label: t("crumbs.revealFolder"), run: () => project.revealInTree(dir, true) });
    openMenuBelow(el, items);
  }

  function fileMenu(el: Element, path: string) {
    const r = project.remote;
    const web = r && rel && project.git?.branch ? fileUrl(r, project.git.branch, rel) : null;
    openMenuBelow(el, [
      { label: t("crumbs.revealFile"), run: () => project.revealInTree(path) },
      ...(web && r ? [{ label: t("forge.open", { name: forgeName(r) }), disabled: !project.git?.upstream, run: () => openUrl(web) }] : []),
      "sep",
      { label: t("menu.copyPath"), run: () => copyText(path) },
      { label: t("menu.copyRelative"), run: () => copyText(rel ?? path) },
      { label: revealLabel(), run: () => reveal(path) },
    ]);
  }

  /** Right-click on a breadcrumb (the project, a folder or the file): where it is, and its path. */
  function crumbMenu(e: MouseEvent, path: string, file: boolean) {
    openMenu(e, [
      { label: revealLabel(), run: () => reveal(path) },
      file
        ? { label: t("crumbs.revealFile"), run: () => project.revealInTree(path) }
        : { label: t("crumbs.revealFolder"), disabled: path === project.root, run: () => project.revealInTree(path, true) },
      "sep",
      { label: t("menu.copyPath"), run: () => copyText(path) },
      { label: t("menu.copyRelative"), disabled: path === project.root, run: () => copyText(relative(project.root, path) ?? path) },
    ]);
  }

  const tabIcon = (tb: Tab) => (tb.kind === "file" ? fileIcon(tb.title) : { icon: ICON[tb.kind], color: undefined });

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
    <div class="tabs" role="tablist" use:reorder={(from, to) => moveItem(project.tabs, from, to)}>
      {#each project.tabs as tb (tb.id)}
        <div class="tab" class:on={tb.id === project.active} role="tab" aria-selected={tb.id === project.active} tabindex="-1" data-reorder>
          <button class="tab-main" onclick={() => (project.active = tb.id)} onauxclick={(e) => onAux(e, tb.id)} oncontextmenu={(e) => tabMenu(e, tb)} title={tb.path}>
            <Icon name={tabIcon(tb).icon} color={tabIcon(tb).color} size={13} />
            <span class="tab-name ellipsis">{tb.title}</span>
          </button>
          <button class="tab-close" data-no-reorder class:dirty={isDirty(tb)} onclick={() => project.closeTab(tb.id)} aria-label={t("tabs.close")} title={t("tabs.close")}>
            <span class="dot"></span><Icon name="close" size={12} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if tab}
    {#if crumbs.length}
      <nav class="crumbs" aria-label={t("crumbs.label")}>
        <button
          class="crumb"
          onclick={(e) => folderMenu(e.currentTarget, project.root)}
          oncontextmenu={(e) => crumbMenu(e, project.root, false)}
          title={project.root}
        >
          {project.current.name}
        </button>
        {#each crumbs as c (c.path)}
          <Icon name="right" size={11} class="crumb-sep" />
          {#if c.last}
            <button
              class="crumb file"
              onclick={(e) => fileMenu(e.currentTarget, c.path)}
              oncontextmenu={(e) => crumbMenu(e, c.path, true)}
              title={c.path}
            >
              <Icon name={fileIcon(c.name).icon} color={fileIcon(c.name).color} size={12} />{c.name}
            </button>
          {:else}
            <button
              class="crumb"
              onclick={(e) => folderMenu(e.currentTarget, c.path)}
              oncontextmenu={(e) => crumbMenu(e, c.path, false)}
              title={c.path}
            >
              {c.name}
            </button>
          {/if}
        {/each}
      </nav>
    {/if}
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
    {:else if tab.kind === "preview"}
      {#key tab.id}
        <PreviewView {tab} />
      {/key}
    {:else}
      {#key tab.id}
        <DiffView {tab} />
      {/key}
    {/if}
  {:else}
    <div class="welcome">
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
    /* A tall terminal squeezes this area; nothing in it may spill over the terminal. */
    overflow: hidden;
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
  .tab:global(.dragging) {
    position: relative;
    z-index: 2;
    opacity: 0.85;
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
  .crumbs {
    flex: none;
    display: flex;
    align-items: center;
    gap: 1px;
    height: 26px;
    padding: 0 8px;
    overflow-x: auto;
    scrollbar-width: none;
    border-bottom: 1px solid var(--line);
    background: var(--canvas);
  }
  .crumbs::-webkit-scrollbar {
    display: none;
  }
  .crumbs :global(.crumb-sep) {
    flex: none;
    color: var(--muted);
    opacity: 0.6;
  }
  .crumb {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 5px;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .crumb:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .crumb.file {
    color: var(--text);
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
  .spacer {
    flex: 1;
  }
  .welcome {
    flex: 1;
    min-height: 0;
    display: flex;
    /* Centered while it fits; once it doesn't, it stays at the top and the terminal covers the rest. */
    align-items: safe center;
    justify-content: center;
    padding: 24px;
    overflow: hidden;
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
