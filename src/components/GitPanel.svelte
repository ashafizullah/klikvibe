<script lang="ts">
  import { useProject } from "../lib/project.svelte";
  import { ago } from "../lib/time";
  import { i18n, t, type Key } from "../lib/i18n/index.svelte";
  import { mod } from "../lib/ui.svelte";
  import Icon from "./Icon.svelte";

  const project = useProject();

  const git = $derived(project.git);
  const MARK: Record<string, string> = { added: "A", modified: "M", deleted: "D", renamed: "R", untracked: "U", conflict: "!" };

  function setTab(tab: "changes" | "history") {
    project.gitTab = tab;
    if (tab === "history") project.refreshGit(true);
  }

  function onKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      project.commitAll();
    }
  }

  const split = (path: string) => {
    const i = path.lastIndexOf("/");
    return i < 0 ? { name: path, dir: "" } : { name: path.slice(i + 1), dir: path.slice(0, i) };
  };
</script>

<section class="git">
  <div class="panel-head">
    <span class="panel-title">Git</span>
    <button class="icon-btn" onclick={() => project.refreshGit(true)} title={t("git.refresh")} aria-label={t("git.refresh")}><Icon name="refresh" size={13} /></button>
  </div>

  {#if !git}
    <p class="empty-note">{t("common.loading")}</p>
  {:else if !git.repo}
    <div class="norepo">
      <Icon name="branch" size={26} stroke={1.5} />
      <p>{t("git.notRepo")}</p>
      <button class="btn primary" onclick={project.initRepo} disabled={!!project.gitBusy}><Icon name="plus" size={13} />{t("git.init")}</button>
    </div>
  {:else}
    <div class="sync">
      <div class="branch">
        <Icon name="branch" size={13} />
        <span class="mono">{git.branch ?? t("git.detached")}</span>
        {#if git.upstream}<span class="upstream mono">→ {git.upstream}</span>{/if}
      </div>
      <div class="sync-buttons">
        <button class="btn small" onclick={project.pull} disabled={!!project.gitBusy || !git.upstream} title={t("git.pullHint")}>
          <Icon name={project.gitBusy === "pull" ? "loader" : "pull"} size={13} class={project.gitBusy === "pull" ? "spin" : ""} />{t("git.pull")}
          {#if git.behind}<span class="count">{git.behind}</span>{/if}
        </button>
        <button class="btn small" class:primary={git.ahead > 0} onclick={project.push} disabled={!!project.gitBusy} title={git.upstream ? t("git.pushHint") : t("git.publishHint")}>
          <Icon name={project.gitBusy === "push" ? "loader" : "push"} size={13} class={project.gitBusy === "push" ? "spin" : ""} />{git.upstream ? t("git.push") : t("git.publish")}
          {#if git.ahead}<span class="count">{git.ahead}</span>{/if}
        </button>
      </div>
    </div>

    <div class="segmented tabs" role="tablist">
      <button class:on={project.gitTab === "changes"} onclick={() => setTab("changes")} role="tab" aria-selected={project.gitTab === "changes"}>
        {t("git.changes")}{#if git.changes.length}&nbsp;<span class="count">{git.changes.length}</span>{/if}
      </button>
      <button class:on={project.gitTab === "history"} onclick={() => setTab("history")} role="tab" aria-selected={project.gitTab === "history"}>{t("git.history")}</button>
    </div>

    {#if project.gitTab === "changes"}
      <div class="commit">
        <textarea
          class="field"
          rows="2"
          placeholder={t("git.messagePlaceholder")}
          bind:value={project.commitMessage}
          onkeydown={onKey}
          spellcheck="false"
        ></textarea>
        <button class="btn primary" onclick={() => project.commitAll()} disabled={!!project.gitBusy || !git.changes.length || !project.commitMessage.trim()} title={`${t("git.commitHint")} (${mod}↵)`}>
          <Icon name={project.gitBusy === "commit" ? "loader" : "check"} size={13} class={project.gitBusy === "commit" ? "spin" : ""} />{t("git.commitAll")}
        </button>
      </div>
      <div class="list">
        {#each git.changes as c (c.path)}
          {@const p = split(c.path)}
          <button class="change" onclick={() => project.openDiff(c)} title={c.from ? `${c.from} → ${c.path}` : c.path}>
            <span class="name">{p.name}</span>
            <span class="dir">{p.dir}</span>
            <span class="git-mark {c.kind}" title={t(`git.kind.${c.kind}` as Key)}>{MARK[c.kind]}</span>
          </button>
        {:else}
          <p class="empty-note clean"><Icon name="check" size={14} />{t("git.clean")}</p>
        {/each}
      </div>
    {:else}
      <div class="list">
        {#each project.log as c (c.hash)}
          <button class="commit-row" onclick={() => project.openCommit(c)} title={c.hash}>
            <span class="subject">{c.subject}</span>
            <span class="meta">
              {#if c.local}<span class="local" title={t("git.notPushed")}><Icon name="arrow-up" size={10} /></span>{/if}
              <span class="mono">{c.short}</span> · {c.author} · {ago(c.date, i18n.locale)}
            </span>
          </button>
        {:else}
          <p class="empty-note">{t("git.noCommits")}</p>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .git {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    min-width: 0;
  }
  .norepo {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 32px 16px;
    color: var(--muted);
    text-align: center;
    font-size: 12px;
  }
  .norepo p {
    margin: 0;
  }
  .sync {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--line);
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    min-width: 0;
  }
  .branch :global(svg) {
    color: var(--accent);
    flex: none;
  }
  .upstream {
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sync-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .small {
    height: 26px;
  }
  .count {
    font: 500 10px var(--mono);
    padding: 0 5px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 12%, transparent);
  }
  .tabs {
    margin: 10px 12px 0;
    display: flex;
  }
  .tabs button {
    flex: 1;
    padding: 5px 8px;
  }
  .commit {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .commit textarea {
    resize: none;
    font-family: var(--sans);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 12px;
  }
  .change {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px 0 12px;
    border: none;
    background: none;
    text-align: left;
    font-size: 12.5px;
  }
  .change:hover,
  .commit-row:hover {
    background: var(--panel-2);
  }
  .name {
    flex: none;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11px;
    direction: rtl;
    text-align: left;
  }
  .clean {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .clean :global(svg) {
    color: var(--ok);
  }
  .commit-row {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 12px;
    border: none;
    border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
    background: none;
    text-align: left;
  }
  .subject {
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .local {
    display: inline-flex;
    color: var(--accent);
  }
</style>
