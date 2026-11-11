<script lang="ts">
  // A dev server's page in a tab, with its address, a reload and a way out to the browser.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Tab } from "../lib/project.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { tab }: { tab: Tab } = $props();
  // svelte-ignore state_referenced_locally
  let address = $state(tab.path);
  // The tab can be sent to another address from outside (its link in a terminal).
  $effect(() => {
    address = tab.path;
  });

  /** Only local addresses: the frame is allowed to show nothing else. */
  const local = (u: string) => /^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?(\/|$)/.test(u);

  function go(e: SubmitEvent) {
    e.preventDefault();
    const u = /^https?:\/\//.test(address) ? address : `http://${address}`;
    if (!local(u)) {
      openUrl(u).catch(() => {});
      address = tab.path;
      return;
    }
    tab.path = address = u;
    tab.revision++;
  }
</script>

<div class="preview">
  <form class="bar" onsubmit={go}>
    <button type="button" class="icon-btn" onclick={() => tab.revision++} title={t("preview.reload")} aria-label={t("preview.reload")}><Icon name="refresh" size={13} /></button>
    <input class="field mono" bind:value={address} spellcheck="false" aria-label={t("preview.address")} />
    <button type="button" class="icon-btn" onclick={() => openUrl(tab.path).catch(() => {})} title={t("preview.browser")} aria-label={t("preview.browser")}>
      <Icon name="external" size={13} />
    </button>
  </form>
  {#key tab.revision}
    <iframe src={tab.path} title={tab.title}></iframe>
  {/key}
</div>

<style>
  .preview {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    border-bottom: 1px solid var(--line);
  }
  .bar .field {
    flex: 1;
    height: 26px;
    font-size: 12px;
  }
  iframe {
    flex: 1;
    border: none;
    /* Pages that don't set a background expect white. */
    background: #fff;
  }
</style>
