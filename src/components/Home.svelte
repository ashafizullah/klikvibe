<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { errorText, ipc, type RecentProject } from "../lib/ipc";
  import { openProject, workspace } from "../lib/project.svelte";
  import { AGENTS } from "../lib/agents";
  import { copyText, isMac, mod, openMenu, reveal, revealLabel, toast, ui } from "../lib/ui.svelte";
  import { checkForUpdates, updates } from "../lib/updates.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import LanguageToggle from "./LanguageToggle.svelte";

  let version = $state("");
  let recent = $state<RecentProject[]>([]);

  const load = () =>
    ipc
      .recentProjects()
      .then((r) => (recent = r))
      .catch(() => {});

  onMount(() => {
    getVersion()
      .then((v) => (version = v))
      .catch(() => {});
    load();
  });

  async function pick() {
    try {
      const p = await ipc.pickProject();
      if (p) await openProject(p);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function reopen(r: RecentProject) {
    try {
      await openProject(await ipc.openRecent(r.root));
    } catch (e) {
      toast(errorText(e), "error");
      load();
    }
  }

  function menu(e: MouseEvent, r: RecentProject) {
    openMenu(e, [
      { label: t("menu.open"), disabled: !r.exists, run: () => reopen(r) },
      "sep",
      { label: t("menu.copyPath"), run: () => copyText(r.root) },
      { label: revealLabel(), disabled: !r.exists, run: () => reveal(r.root) },
      "sep",
      { label: t("home.forget"), run: () => forget(r) },
    ]);
  }

  async function forget(r: RecentProject) {
    await ipc.forgetRecent(r.root).catch(() => {});
    load();
  }
</script>

<div class="home">
  <aside>
    {#if isMac && !workspace.projects.length}<div class="traffic" data-tauri-drag-region></div>{/if}
    <div class="brand" data-tauri-drag-region>
      <img src="/icon.png" alt="" width="32" height="32" draggable="false" />
      <span class="wordmark">Klik<em>Vibe</em></span>
    </div>

    <span class="label section">{t("home.start")}</span>
    <nav>
      <button class="nav-item" onclick={pick}>
        <Icon name="open" size={15} />
        <span class="nav-text">
          <span>{t("home.open")}</span>
          <span class="nav-hint">{mod}O</span>
        </span>
      </button>
      <button class="nav-item" onclick={() => (ui.dialog = "mcp")}>
        <Icon name="plug" size={15} />
        <span class="nav-text">
          <span>{t("mcp.title")}</span>
          <span class="nav-hint">{t("mcp.navHint")}</span>
        </span>
      </button>
    </nav>

    <span class="label section">{t("home.agents")}</span>
    <ul class="agents">
      {#each AGENTS as a (a.id)}
        <li title={a.command}>{a.label}</li>
      {/each}
    </ul>
    <p class="agents-hint">{t("home.agentsHint")}</p>

    <div class="grow"></div>
    <div class="foot">
      <button class="version" onclick={checkForUpdates} disabled={updates.checking} title={t("update.check")}>
        <span>KlikVibe {version && `v${version}`}</span>
        <span class="check"><Icon name="refresh" size={10} class={updates.checking ? "spin" : ""} />{t("update.check")}</span>
      </button>
      <LanguageToggle />
    </div>
  </aside>

  <main>
    <header data-tauri-drag-region>
      <div class="titles" data-tauri-drag-region>
        <h1>{t("home.title")}</h1>
        <p class="sub">{t("home.tagline")}</p>
      </div>
      <button class="btn primary" onclick={pick}><Icon name="open" size={14} />{t("home.open")}</button>
    </header>

    <div class="body">
      <section class="hero rise">
        <div class="hero-icon"><Icon name="sparkles" size={22} /></div>
        <div class="hero-text">
          <h2>{t("home.heroTitle")}</h2>
          <p>{t("home.heroText")}</p>
        </div>
      </section>

      <section>
        <h2 class="label">{t("home.recent")}</h2>
        {#if recent.length}
          <div class="list">
            {#each recent as r, i (r.root)}
              <div class="row rise" class:missing={!r.exists} role="listitem" oncontextmenu={(e) => menu(e, r)} style:animation-delay="{Math.min(i, 12) * 40}ms">
                <button class="row-main" onclick={() => reopen(r)} disabled={!r.exists} title={r.root}>
                  <Icon name="project" size={16} />
                  <span class="name">{r.name}</span>
                  <span class="path">{r.exists ? r.root : t("home.missing")}</span>
                </button>
                <button class="icon-btn forget" onclick={() => forget(r)} title={t("home.forget")} aria-label={t("home.forget")}>
                  <Icon name="close" size={13} />
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <div class="empty">
            <Icon name="project" size={32} stroke={1.5} />
            <p>{t("home.recentEmpty")}</p>
          </div>
        {/if}
      </section>
    </div>
  </main>
</div>

<style>
  .home {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  aside {
    width: 256px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-right: 1px solid var(--line);
  }
  .traffic {
    height: 30px;
    flex: none;
  }
  .brand {
    height: 56px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
  }
  .brand img {
    border-radius: 7px;
  }
  .wordmark {
    font: 500 18px var(--serif);
    letter-spacing: -0.02em;
  }
  .wordmark em {
    color: var(--accent);
  }
  .section {
    padding: 16px 16px 4px;
    margin: 0;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
  }
  .nav-item {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 8px;
    border: none;
    border-radius: 6px;
    background: none;
    text-align: left;
    color: var(--muted);
    transition:
      background 0.15s,
      color 0.15s;
  }
  .nav-item :global(svg) {
    margin-top: 1px;
    flex: none;
  }
  .nav-item:hover {
    background: color-mix(in srgb, var(--panel-2) 70%, transparent);
    color: var(--accent);
  }
  .nav-text {
    display: flex;
    flex-direction: column;
    font-size: 12px;
    color: var(--text);
  }
  .nav-hint {
    font: 10.5px var(--mono);
    color: var(--muted);
  }
  .agents {
    list-style: none;
    margin: 0;
    padding: 2px 16px;
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    font-size: 11px;
  }
  .agents li {
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 999px;
    color: var(--text);
  }
  .agents-hint {
    margin: 8px 16px 0;
    font-size: 10.5px;
    line-height: 1.45;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 12px;
    border-top: 1px solid var(--line);
  }
  .version {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 0;
    border: none;
    background: none;
    font-size: 10px;
    color: var(--muted);
    text-align: left;
  }
  .version:hover {
    color: var(--text);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    height: 64px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 24px;
    border-bottom: 1px solid var(--line);
  }
  .titles {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font: 500 20px var(--serif);
    letter-spacing: -0.015em;
  }
  .sub {
    margin: 0;
    font: 10.5px var(--mono);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: linear-gradient(
      to bottom right,
      color-mix(in srgb, var(--accent) 15%, transparent),
      var(--panel) 50%,
      color-mix(in srgb, var(--accent-2) 10%, transparent)
    );
  }
  .hero-icon {
    width: 44px;
    height: 44px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }
  .hero-text {
    flex: 1;
    min-width: 0;
  }
  .hero h2 {
    margin: 0;
    font: 500 20px var(--serif);
    letter-spacing: -0.015em;
  }
  .hero p {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  section > .label {
    margin-bottom: 12px;
  }
  .list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 10px;
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .row:last-child {
    border-bottom: none;
  }
  .row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: none;
    text-align: left;
  }
  .row-main:hover:not(:disabled) {
    background: var(--panel-2);
  }
  .row-main :global(svg) {
    color: var(--accent);
    flex: none;
  }
  .row.missing .row-main {
    opacity: 0.5;
    cursor: default;
  }
  .name {
    font-weight: 500;
    font-size: 13px;
    flex: none;
  }
  .path {
    font: 11px var(--mono);
    color: var(--muted);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .forget {
    margin-right: 6px;
    color: var(--muted);
    opacity: 0;
  }
  .row:hover .forget {
    opacity: 1;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 40px 0;
    color: var(--muted);
    font-size: 12px;
  }
  .empty p {
    margin: 0;
  }
</style>
