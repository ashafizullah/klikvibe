<script lang="ts">
  import { parseDiff, stats } from "../lib/diff";
  import { openFile, type Tab } from "../lib/project.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { tab }: { tab: Tab } = $props();

  const rows = $derived(parseDiff(tab.text));
  const count = $derived(stats(rows));
</script>

<div class="diff grain-free">
  <div class="head">
    <Icon name={tab.kind === "commit" ? "commit" : "diff"} size={14} />
    <span class="title">{tab.kind === "commit" ? t("diff.commit", { hash: tab.title }) : t("diff.changes", { name: tab.title })}</span>
    <span class="stat add">+{count.added}</span>
    <span class="stat del">−{count.removed}</span>
    <div class="spacer"></div>
    {#if tab.kind === "diff"}
      <button class="btn ghost small" onclick={() => openFile(tab.path)}><Icon name="code" size={13} />{t("diff.openFile")}</button>
    {/if}
  </div>
  <div class="rows">
    {#if tab.loading}
      <p class="empty-note">{t("common.loading")}</p>
    {:else if !rows.length}
      <p class="empty-note">{t("diff.none")}</p>
    {/if}
    {#each rows as row, i (i)}
      {#if row.kind === "file"}
        <div class="file"><Icon name="code" size={13} />{row.text}</div>
      {:else if row.kind === "meta"}
        <div class="meta">{row.text}</div>
      {:else if row.kind === "hunk"}
        <div class="hunk">{row.text}</div>
      {:else}
        <div class="line {row.kind}">
          <span class="no">{row.old ?? ""}</span>
          <span class="no">{row.new ?? ""}</span>
          <span class="sign">{row.kind === "add" ? "+" : row.kind === "del" ? "−" : " "}</span>
          <span class="text">{row.text}</span>
        </div>
      {/if}
    {/each}
  </div>
</div>

<style>
  .diff {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
  }
  .head {
    height: 36px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  .head :global(svg) {
    color: var(--muted);
  }
  .title {
    font-weight: 500;
  }
  .stat {
    font: 11px var(--mono);
  }
  .stat.add {
    color: var(--ok);
  }
  .stat.del {
    color: var(--danger);
  }
  .spacer {
    flex: 1;
  }
  .small {
    height: 24px;
  }
  .rows {
    flex: 1;
    overflow: auto;
    font: 12.5px/1.6 var(--mono);
    padding-bottom: 24px;
    -webkit-user-select: text;
    user-select: text;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 14px 4px;
    color: var(--accent);
    font-weight: 500;
    position: sticky;
    top: 0;
    background: var(--canvas);
  }
  .meta {
    padding: 0 14px;
    color: var(--muted);
    white-space: pre-wrap;
  }
  .hunk {
    padding: 2px 14px;
    margin-top: 6px;
    color: #8b7fb8;
    background: color-mix(in srgb, #7c3aed 8%, transparent);
    white-space: pre;
  }
  .line {
    display: flex;
    white-space: pre;
    min-width: max-content;
  }
  .line.add {
    background: color-mix(in srgb, var(--ok) 13%, transparent);
  }
  .line.del {
    background: color-mix(in srgb, var(--danger) 13%, transparent);
  }
  .no {
    width: 44px;
    flex: none;
    padding-right: 8px;
    text-align: right;
    color: #5d5547;
  }
  .sign {
    width: 16px;
    flex: none;
    color: var(--muted);
  }
  .line.add .sign {
    color: var(--ok);
  }
  .line.del .sign {
    color: var(--danger);
  }
  .text {
    padding-right: 24px;
  }
</style>
