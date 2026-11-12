<script lang="ts">
  // The command palette (⌘⇧P / Ctrl+Shift+P): type to find a command, Enter runs it.
  import { onMount, tick } from "svelte";
  import { ipc, type RecentProject } from "../lib/ipc";
  import { commands, type Command } from "../lib/commands.svelte";
  import { rank } from "../lib/palette";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  const RECENT_KEY = "klikvibe.palette.recent";
  /** Commands remembered as used last, shown first while nothing is typed. */
  const RECENT_MAX = 6;

  let recentIds: string[] = [];
  try {
    recentIds = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
  } catch {}

  let recentProjects = $state<RecentProject[]>([]);
  let query = $state("");
  let selected = $state(0);
  let input = $state<HTMLInputElement>();
  let listEl = $state<HTMLElement>();

  const all = $derived(commands(recentProjects).map((c) => ({ ...c, text: `${c.group}: ${c.label}` })));
  const shown = $derived(rank(all, query, recentIds));

  onMount(() => {
    input?.focus();
    ipc
      .recentProjects()
      .then((r) => (recentProjects = r))
      .catch(() => {});
  });

  const close = () => (ui.dialog = null);

  function run(c: Command) {
    close();
    const ids = [c.id, ...recentIds.filter((id) => id !== c.id)].slice(0, RECENT_MAX);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(ids));
    } catch {}
    // After the palette is gone, so a command that puts the cursor somewhere keeps it there.
    tick().then(() => c.run());
  }

  async function select(i: number) {
    selected = (i + shown.length) % shown.length;
    await tick();
    listEl?.querySelector(".on")?.scrollIntoView({ block: "nearest" });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (!shown.length) {
      return;
    } else if (e.key === "ArrowDown" || (e.ctrlKey && e.key === "n")) {
      e.preventDefault();
      select(selected + 1);
    } else if (e.key === "ArrowUp" || (e.ctrlKey && e.key === "p")) {
      e.preventDefault();
      select(selected - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      run(shown[Math.min(selected, shown.length - 1)]);
    }
  }
</script>

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="palette rise" role="dialog" aria-modal="true" aria-label={t("palette.title")}>
    <div class="field">
      <Icon name="search" size={14} />
      <input
        bind:this={input}
        bind:value={query}
        oninput={() => (selected = 0)}
        {onkeydown}
        placeholder={t("palette.placeholder")}
        spellcheck="false"
        autocomplete="off"
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-list"
        aria-activedescendant={shown.length ? `palette-${selected}` : undefined}
      />
    </div>
    {#if shown.length}
      <ul id="palette-list" class="list" role="listbox" bind:this={listEl}>
        {#each shown as c, i (c.id)}
          <li id={`palette-${i}`} role="option" aria-selected={i === selected} class:on={i === selected}>
            <button tabindex="-1" onclick={() => run(c)} onmousemove={() => (selected = i)}>
              <Icon name={c.icon} size={14} />
              <span class="group">{c.group}</span>
              <span class="name ellipsis">{c.label}</span>
              {#if c.keys}<span class="kbd">{c.keys}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="none">{t("palette.none")}</p>
    {/if}
    <div class="foot">{t("palette.keys")}</div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 12vh 24px 24px;
    background: rgb(0 0 0 / 0.35);
  }
  .palette {
    width: min(620px, 100%);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .field {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  .field input {
    flex: 1;
    height: 44px;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
  }
  .list button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }
  .list button :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .on button {
    background: var(--panel-2);
  }
  .on button :global(svg) {
    color: var(--accent);
  }
  .group {
    flex: none;
    color: var(--muted);
    font-size: 12px;
  }
  /* Not .label: that is the small upper-case heading style (lists.css). */
  .name {
    flex: 1;
    min-width: 0;
  }
  .kbd {
    flex: none;
  }
  .none {
    margin: 0;
    padding: 18px 16px;
    color: var(--muted);
    font-size: 13px;
  }
  .foot {
    flex: none;
    padding: 7px 14px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 11.5px;
  }
</style>
