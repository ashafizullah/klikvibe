<script lang="ts">
  // A preview of an image, video, audio file or PDF, loaded over media://.
  // It reloads when the file changes on disk (the tab's revision).
  import type { Tab } from "../lib/project.svelte";
  import { mediaUrl } from "../lib/media";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { tab, onopen }: { tab: Tab; onopen: () => void } = $props();

  const src = $derived(mediaUrl(tab.path, tab.revision));
  let failed = $state(false);
  let actual = $state(false);
  let size = $state<{ w: number; h: number } | null>(null);

  // A new version of the file gets a fresh try.
  $effect(() => {
    void src;
    failed = false;
  });

  function loaded(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    size = img.naturalWidth ? { w: img.naturalWidth, h: img.naturalHeight } : null;
  }
</script>

<div class="media">
  {#if failed}
    <div class="notice">
      <Icon name={tab.media ?? "file"} size={28} stroke={1.5} />
      <p>{t("media.error")}</p>
      <button class="btn small" onclick={onopen}><Icon name="external" size={13} />{t("media.openExternally")}</button>
    </div>
  {:else if tab.media === "image"}
    <button class="stage checker" class:actual onclick={() => (actual = !actual)} title={actual ? t("media.fit") : t("media.actualSize")}>
      <img {src} alt={tab.title} onload={loaded} onerror={() => (failed = true)} draggable="false" />
    </button>
    {#if size}<span class="dims">{size.w} × {size.h}</span>{/if}
  {:else if tab.media === "video"}
    <div class="stage">
      <!-- svelte-ignore a11y_media_has_caption -->
      <video {src} controls preload="metadata" onerror={() => (failed = true)}></video>
    </div>
  {:else if tab.media === "audio"}
    <div class="stage">
      <div class="audio">
        <Icon name="audio" size={40} stroke={1.25} />
        <span class="name">{tab.title}</span>
        <audio {src} controls preload="metadata" onerror={() => (failed = true)}></audio>
      </div>
    </div>
  {:else if tab.media === "pdf"}
    <iframe {src} title={tab.title}></iframe>
  {/if}
</div>

<style>
  .media {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    background: var(--bg);
  }
  .stage {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    overflow: hidden;
    border: none;
    border-radius: 0;
    background: none;
    color: inherit;
  }
  button.stage {
    cursor: zoom-in;
  }
  /* A checkerboard behind images, so transparent parts show as transparent. */
  .checker {
    --sq: color-mix(in srgb, var(--line) 55%, transparent);
    background:
      conic-gradient(var(--sq) 25%, transparent 0 50%, var(--sq) 0 75%, transparent 0) 0 0 / 20px 20px,
      var(--bg);
  }
  .stage img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    box-shadow: 0 0 0 1px var(--line);
  }
  .stage.actual {
    cursor: zoom-out;
    overflow: auto;
    /* Centered while it fits, scrollable from the top-left once it doesn't. */
    align-items: safe center;
    justify-content: safe center;
  }
  .stage.actual img {
    max-width: none;
    max-height: none;
    flex: none;
  }
  .dims {
    position: absolute;
    right: 12px;
    bottom: 10px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    font: 11px var(--mono);
    color: var(--muted);
    pointer-events: none;
  }
  video {
    max-width: 100%;
    max-height: 100%;
    background: #000;
    border-radius: 6px;
  }
  .audio {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    color: var(--muted);
  }
  .audio .name {
    font: 500 15px var(--serif);
    color: var(--text);
  }
  .audio audio {
    width: min(420px, 80vw);
  }
  iframe {
    flex: 1;
    border: none;
    background: var(--panel);
  }
  .notice {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    color: var(--muted);
    font-size: 12px;
  }
</style>
