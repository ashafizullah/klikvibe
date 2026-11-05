<script lang="ts" generics="T extends string | boolean">
  // A row of mutually exclusive choices (a radio group): Names / Contents,
  // This branch / All branches, the diff layout, the language.
  import Icon from "./Icon.svelte";

  let {
    options,
    value,
    onchange,
    label,
    stretch = false,
    keepFocus = false,
  }: {
    options: { value: T; label?: string; icon?: string; title?: string }[];
    value: T;
    onchange: (value: T) => void;
    /** What the choice is about, for screen readers. */
    label: string;
    /** Fill the width, the choices sharing it equally. */
    stretch?: boolean;
    /** Clicking doesn't take the focus away (e.g. from a search field). */
    keepFocus?: boolean;
  } = $props();
</script>

<div class="segmented" class:stretch role="radiogroup" aria-label={label}>
  {#each options as o (String(o.value))}
    <button
      class:on={o.value === value}
      class:icon={!o.label}
      role="radio"
      aria-checked={o.value === value}
      aria-label={o.label ? undefined : o.title}
      title={o.title}
      onmousedown={(e) => keepFocus && e.preventDefault()}
      onclick={() => onchange(o.value)}
    >
      {#if o.icon}<Icon name={o.icon} size={13} />{/if}
      {#if o.label}{o.label}{/if}
    </button>
  {/each}
</div>
