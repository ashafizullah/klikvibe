<script lang="ts" module>
  /** Open dialogs, newest last: Escape closes only the one on top. */
  const stack: symbol[] = [];
</script>

<script lang="ts">
  // A modal: the dimmed backdrop, the panel, Escape and a click outside to close.
  // What goes inside (header, body, buttons) is the caller's.
  import { onMount, type Snippet } from "svelte";

  let {
    onclose,
    width = 520,
    padding = 0,
    gap = 0,
    alert = false,
    labelledby,
    describedby,
    panel = $bindable(),
    children,
  }: {
    onclose: () => void;
    /** Largest width in pixels; narrower windows get the full width. */
    width?: number;
    padding?: number;
    gap?: number;
    /** An alert (role alertdialog) stacks above other dialogs. */
    alert?: boolean;
    labelledby?: string;
    describedby?: string;
    panel?: HTMLDivElement;
    children: Snippet;
  } = $props();

  const me = Symbol("dialog");
  onMount(() => {
    stack.push(me);
    return () => stack.splice(stack.indexOf(me), 1);
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && stack[stack.length - 1] === me) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" class:alert role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div
    class="dialog rise"
    role={alert ? "alertdialog" : "dialog"}
    aria-modal="true"
    aria-labelledby={labelledby}
    aria-describedby={describedby}
    style:width="min({width}px, 100%)"
    style:padding="{padding}px"
    style:gap="{gap}px"
    bind:this={panel}
  >
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(0 0 0 / 0.45);
  }
  .backdrop.alert {
    z-index: 130;
  }
  .dialog {
    position: relative;
    max-height: calc(100vh - 48px);
    overflow: auto;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
  }
</style>
