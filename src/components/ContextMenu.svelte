<script lang="ts">
  // The right-click menu (see openMenu in ui.svelte.ts). It stays inside the
  // window, closes on a click elsewhere, Escape, scrolling or resizing, and
  // works from the keyboard.
  import { tick } from "svelte";
  import { closeMenu, ui, type MenuItem } from "../lib/ui.svelte";
  import Icon from "./Icon.svelte";

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  const buttons = () => [...(el?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];

  $effect(() => {
    const menu = ui.menu;
    if (!menu) return;
    pos = { x: menu.x, y: menu.y };
    tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = { x: Math.max(4, Math.min(menu.x, innerWidth - r.width - 4)), y: Math.max(4, Math.min(menu.y, innerHeight - r.height - 4)) };
      buttons()[0]?.focus();
    });
  });

  function run(item: Exclude<MenuItem, "sep">) {
    closeMenu();
    item.run();
  }

  function onKey(e: KeyboardEvent) {
    if (!ui.menu) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeMenu();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const list = buttons();
      const i = list.indexOf(document.activeElement as HTMLButtonElement);
      const next = e.key === "ArrowDown" ? (i + 1) % list.length : (i - 1 + list.length) % list.length;
      list[next]?.focus();
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (ui.menu && !el?.contains(e.target as Node)) closeMenu();
  }
</script>

<svelte:window onkeydowncapture={onKey} onpointerdowncapture={onPointerDown} onresize={closeMenu} onblur={closeMenu} onwheelcapture={() => ui.menu && closeMenu()} />

{#if ui.menu}
  <div class="menu" role="menu" tabindex="-1" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" oncontextmenu={(e) => e.preventDefault()}>
    {#each ui.menu.items as item, i (i)}
      {#if item === "sep"}
        <div class="sep" role="separator"></div>
      {:else}
        <button role="menuitem" disabled={item.disabled} onclick={() => run(item)}>
          {#if item.icon}<Icon name={item.icon} color={item.color} size={13} />{/if}
          <span class="label">{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 150;
    min-width: 200px;
    max-width: 360px;
    /* A long folder listing scrolls instead of leaving the window. */
    max-height: min(480px, calc(100vh - 16px));
    overflow-y: auto;
    padding: 4px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: var(--shadow);
    outline: none;
  }
  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text);
    font-size: 12.5px;
    text-align: left;
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    outline: none;
  }
  button:disabled {
    color: var(--muted);
    opacity: 0.6;
  }
  button :global(svg) {
    flex: none;
    margin-right: -8px;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    margin-left: auto;
    font: 11px var(--mono);
    color: var(--muted);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
</style>
