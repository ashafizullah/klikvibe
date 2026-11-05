<script lang="ts">
  // Tooltips for everything with a `title`: the web view on macOS doesn't
  // reliably show native ones. While an element is hovered its title moves to
  // a data attribute (so no native tooltip doubles this one) and comes back
  // when the pointer leaves. A shortcut in trailing parentheses, "Files (⌘B)",
  // is shown as a key.
  import { tick } from "svelte";

  const DELAY = 450;
  /** Moving to a neighbour within this time shows its tooltip at once. */
  const WARM = 400;

  let tip = $state<{ text: string; keys: string } | null>(null);
  let pos = $state({ x: 0, y: 0, below: true });
  let box = $state<HTMLDivElement>();
  let target: HTMLElement | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let hiddenAt = 0;

  function split(title: string) {
    const m = /^(.*\S)\s+\(((?:⌘|⇧|⌥|⌃|Ctrl\+|Shift\+|Alt\+)[^()]{0,20})\)$/s.exec(title);
    return m ? { text: m[1], keys: m[2] } : { text: title, keys: "" };
  }

  function release() {
    if (target?.dataset.tip !== undefined) {
      // Svelte may have set a new title meanwhile; keep that one.
      if (!target.hasAttribute("title")) target.setAttribute("title", target.dataset.tip);
      delete target.dataset.tip;
    }
    target = null;
  }

  function hide() {
    if (timer) clearTimeout(timer);
    timer = null;
    if (tip) hiddenAt = Date.now();
    tip = null;
    release();
  }

  async function show(el: HTMLElement, title: string) {
    tip = split(title);
    await tick();
    if (!box || target !== el) return;
    const r = el.getBoundingClientRect();
    const b = box.getBoundingClientRect();
    const below = r.bottom + 8 + b.height < innerHeight;
    pos = {
      x: Math.max(6, Math.min(r.left + r.width / 2 - b.width / 2, innerWidth - b.width - 6)),
      y: below ? r.bottom + 6 : r.top - b.height - 6,
      below,
    };
  }

  function over(e: PointerEvent) {
    if (e.pointerType === "touch") return;
    const el = (e.target as Element | null)?.closest<HTMLElement>("[title], [data-tip]");
    if (el === target) return;
    hide();
    const title = el?.getAttribute("title");
    if (!el || !title?.trim()) return;
    target = el;
    el.dataset.tip = title;
    el.removeAttribute("title");
    const wait = Date.now() - hiddenAt < WARM ? 0 : DELAY;
    timer = setTimeout(() => show(el, title), wait);
  }

  function out(e: PointerEvent) {
    if (target && !target.contains(e.relatedTarget as Node | null)) hide();
  }
</script>

<svelte:window
  onpointerover={over}
  onpointerout={out}
  onpointerdowncapture={hide}
  onkeydowncapture={hide}
  onwheelcapture={hide}
  onblur={hide}
/>

{#if tip}
  <div class="tip" class:below={pos.below} role="tooltip" bind:this={box} style:left="{pos.x}px" style:top="{pos.y}px">
    <span>{tip.text}</span>
    {#if tip.keys}<span class="keys">{tip.keys}</span>{/if}
  </div>
{/if}

<style>
  .tip {
    position: fixed;
    z-index: 300;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 320px;
    padding: 5px 9px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-2);
    box-shadow: var(--shadow);
    color: var(--text);
    font-size: 11.5px;
    line-height: 1.35;
    white-space: pre-line;
    overflow-wrap: anywhere;
    pointer-events: none;
    animation: tip-in 0.12s ease-out;
  }
  .keys {
    flex: none;
    padding: 1px 5px;
    border: 1px solid var(--line);
    border-radius: 4px;
    background: var(--bg);
    font: 10.5px var(--mono);
    color: var(--muted);
  }
  @keyframes tip-in {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
  .tip:not(.below) {
    animation-name: tip-up;
  }
  @keyframes tip-up {
    from {
      opacity: 0;
      transform: translateY(2px);
    }
  }
</style>
