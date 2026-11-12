<script lang="ts">
  import type { StripScroll } from "../lib/stripScroll.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { strip, dir, size = 24 }: { strip: StripScroll; dir: -1 | 1; size?: number } = $props();
  const label = $derived(t(dir < 0 ? "tabs.scrollLeft" : "tabs.scrollRight"));
</script>

<button
  class="icon-btn scroll"
  style:width="{size - 6}px"
  style:height="{size}px"
  disabled={dir < 0 ? !strip.canLeft : !strip.canRight}
  onclick={() => strip.by(dir)}
  title={label}
  aria-label={label}
>
  <Icon name={dir < 0 ? "left" : "right"} size={14} />
</button>

<style>
  .scroll {
    color: var(--muted);
  }
  .scroll:hover:not(:disabled) {
    color: var(--text);
  }
</style>
