<script lang="ts">
  import { parseDiff, splitDiff, stats, type Half } from "../lib/diff";
  import { persist, settings } from "../lib/settings.svelte";
  import { copyText, mod, openMenu, reveal, revealLabel, type MenuItem } from "../lib/ui.svelte";
  import { relative } from "../lib/paths";
  import { useProject, type Tab } from "../lib/project.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { tab }: { tab: Tab } = $props();
  const project = useProject();

  const rows = $derived(parseDiff(tab.text));
  const count = $derived(stats(rows));
  const split = $derived(settings.diffView === "split");
  const pairs = $derived(split ? splitDiff(rows) : []);

  function setView(view: "split" | "unified") {
    settings.diffView = view;
    persist();
  }

  const SIGN = { add: "+", del: "−", ctx: " " } as const;
  let rowsEl = $state<HTMLDivElement>();

  /** The diff's right-click menu: the selected text, the layout, then the file. */
  function menu(e: MouseEvent) {
    const selected = window.getSelection()?.toString() ?? "";
    const items: MenuItem[] = [
      { label: t("edit.copy"), hint: `${mod}C`, disabled: !selected, run: () => copyText(selected, true) },
      {
        label: t("edit.selectAll"),
        hint: `${mod}A`,
        run: () => {
          if (!rowsEl) return;
          const range = document.createRange();
          range.selectNodeContents(rowsEl);
          window.getSelection()?.removeAllRanges();
          window.getSelection()?.addRange(range);
        },
      },
      "sep",
      { label: split ? t("diff.unified") : t("diff.split"), run: () => setView(split ? "unified" : "split") },
    ];
    if (tab.kind === "commit") {
      items.push("sep", { label: t("menu.copyHash"), run: () => copyText(tab.id.slice("commit:".length)) });
    } else {
      items.push(
        "sep",
        { label: t("diff.openFile"), run: () => project.openFile(tab.path) },
        { label: t("menu.copyPath"), run: () => copyText(tab.path) },
        { label: t("menu.copyRelative"), run: () => copyText(relative(project.root, tab.path) ?? tab.path) },
        { label: revealLabel(), run: () => reveal(tab.path) },
      );
    }
    openMenu(e, items);
  }
</script>

{#snippet half(h: Half | undefined)}
  {#if h}
    <div class="half {h.kind}">
      <span class="no">{h.no}</span>
      <span class="sign">{SIGN[h.kind]}</span>
      {#if h.changed && h.changed[1] > h.changed[0]}
        <span class="text">{h.text.slice(0, h.changed[0])}<mark>{h.text.slice(h.changed[0], h.changed[1])}</mark>{h.text.slice(h.changed[1])}</span>
      {:else}
        <span class="text">{h.text}</span>
      {/if}
    </div>
  {:else}
    <div class="half empty"></div>
  {/if}
{/snippet}

<div class="diff">
  <div class="head">
    <Icon name={tab.kind === "commit" ? "commit" : "diff"} size={14} />
    <span class="title">{tab.kind === "commit" ? t("diff.commit", { hash: tab.title }) : t("diff.changes", { name: tab.title })}</span>
    <span class="stat add">+{count.added}</span>
    <span class="stat del">−{count.removed}</span>
    <div class="spacer"></div>
    <div class="view" role="radiogroup" aria-label={t("diff.view")}>
      <button class:on={split} role="radio" aria-checked={split} onclick={() => setView("split")} title={t("diff.split")}><Icon name="columns" size={13} /></button>
      <button class:on={!split} role="radio" aria-checked={!split} onclick={() => setView("unified")} title={t("diff.unified")}><Icon name="rows" size={13} /></button>
    </div>
    {#if tab.kind === "diff"}
      <button class="btn ghost small" onclick={() => project.openFile(tab.path)}><Icon name="code" size={13} />{t("diff.openFile")}</button>
    {/if}
  </div>
  <div class="rows" bind:this={rowsEl} oncontextmenu={menu} role="presentation">
    {#if tab.loading}
      <p class="empty-note">{t("common.loading")}</p>
    {:else if !rows.length}
      <p class="empty-note">{t("diff.none")}</p>
    {/if}
    {#if split}
      {#each pairs as row, i (i)}
        {#if row.kind === "file"}
          <div class="file"><Icon name="code" size={13} />{row.text}</div>
        {:else if row.kind === "meta"}
          <div class="meta">{row.text}</div>
        {:else if row.kind === "hunk"}
          <div class="hunk">{row.text}</div>
        {:else if row.kind === "pair"}
          <div class="pair">
            {@render half(row.left)}
            {@render half(row.right)}
          </div>
        {/if}
      {/each}
    {:else}
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
    {/if}
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
  .view {
    display: flex;
    gap: 1px;
    padding: 2px;
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .view button {
    width: 24px;
    height: 20px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--muted);
  }
  .view button.on {
    background: var(--panel-2);
    color: var(--text);
  }
  /* Side by side: two equal halves; long lines wrap so both sides stay level. */
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }
  .half {
    display: flex;
    min-width: 0;
  }
  .half + .half {
    border-left: 1px solid var(--line);
  }
  .half .text {
    flex: 1;
    min-width: 0;
    padding-right: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .half.add {
    background: color-mix(in srgb, var(--ok) 13%, transparent);
  }
  .half.del {
    background: color-mix(in srgb, var(--danger) 13%, transparent);
  }
  .half.add .sign {
    color: var(--ok);
  }
  .half.del .sign {
    color: var(--danger);
  }
  .half.empty {
    background: repeating-linear-gradient(135deg, transparent 0 6px, color-mix(in srgb, var(--line) 45%, transparent) 6px 7px);
  }
  mark {
    color: inherit;
    border-radius: 2px;
  }
  .half.add mark {
    background: color-mix(in srgb, var(--ok) 38%, transparent);
  }
  .half.del mark {
    background: color-mix(in srgb, var(--danger) 38%, transparent);
  }
</style>
