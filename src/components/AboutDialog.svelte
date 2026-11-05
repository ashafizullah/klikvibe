<script lang="ts">
  // The version, how updates arrive, and a check on request.
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { settings, type UpdateMode } from "../lib/settings.svelte";
  import { checkForUpdates, installUpdate, offer, setUpdateMode, showCurrentNotes, updates } from "../lib/updates.svelte";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  const REPO = "https://github.com/ashafizullah/klikvibe";
  const MODES: UpdateMode[] = ["auto", "notify", "off"];
  const LABEL = { auto: "about.updateAuto", notify: "about.updateNotify", off: "about.updateOff" } as const;
  const HINT = { auto: "about.updateHint.auto", notify: "about.updateHint.notify", off: "about.updateHint.off" } as const;

  let version = $state("");
  onMount(() => {
    getVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });

  const close = () => (ui.dialog = null);
  const open = (url: string) => openUrl(url).catch(() => {});
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && !ui.alert && close()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="dialog rise" role="dialog" aria-modal="true" aria-labelledby="about-title">
    <button class="icon-btn x" onclick={close} title={t("mcp.close")} aria-label={t("mcp.close")}><Icon name="close" size={14} /></button>

    <div class="who">
      <img src="/icon.png" alt="" width="56" height="56" draggable="false" />
      <div>
        <h2 id="about-title">Klik<em>Vibe</em></h2>
        <p class="ver">{version ? t("about.version", { version }) : ""}</p>
      </div>
    </div>
    <p class="tagline">{t("home.tagline")}</p>

    <section>
      <span class="label">{t("about.updates")}</span>
      <div class="status">
        {#if updates.ready}
          <Icon name="sparkles" size={14} />
          <span>{t("update.ready", { version: updates.ready.version })}</span>
          <button class="btn primary small" onclick={installUpdate}>{t("update.restart")}</button>
        {:else if updates.available}
          <Icon name="sparkles" size={14} />
          <span>{t("update.availableShort", { version: updates.available.version })}</span>
          <button class="btn primary small" onclick={() => updates.available && offer(updates.available)}>{t("update.whatsNew")}</button>
        {:else}
          <span class="muted" class:error={updates.lastResult?.kind === "error"}>{updates.lastResult?.text ?? ""}</span>
          <button class="btn small" onclick={checkForUpdates} disabled={updates.checking}>
            <Icon name={updates.checking ? "loader" : "refresh"} size={13} class={updates.checking ? "spin" : ""} />
            {updates.checking ? t("update.checking") : t("update.check")}
          </button>
        {/if}
      </div>

      <div class="modes" role="radiogroup" aria-label={t("about.updates")}>
        {#each MODES as m (m)}
          <label class="mode" class:on={settings.updateMode === m}>
            <input type="radio" name="update-mode" value={m} checked={settings.updateMode === m} onchange={() => setUpdateMode(m)} />
            {t(LABEL[m])}
          </label>
        {/each}
      </div>
      <p class="hint">{t(HINT[settings.updateMode])}</p>
    </section>

    <div class="links">
      <button class="btn ghost small" onclick={showCurrentNotes}><Icon name="history" size={13} />{t("about.thisVersion")}</button>
      <button class="btn ghost small" onclick={() => open(`${REPO}/releases`)}><Icon name="external" size={13} />{t("about.releases")}</button>
      <button class="btn ghost small" onclick={() => open(REPO)}><Icon name="code" size={13} />{t("about.source")}</button>
    </div>
    <p class="foot">{t("about.license")}</p>
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
  .dialog {
    position: relative;
    width: min(460px, 100%);
    max-height: calc(100vh - 48px);
    overflow: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
  }
  .x {
    position: absolute;
    top: 10px;
    right: 10px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .who img {
    border-radius: 12px;
  }
  h2 {
    margin: 0;
    font: 600 24px var(--serif);
    letter-spacing: -0.02em;
  }
  h2 em {
    font-style: normal;
    color: var(--accent);
  }
  .ver {
    margin: 2px 0 0;
    font: 12px var(--mono);
    color: var(--muted);
  }
  .tagline {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    font-size: 12.5px;
  }
  .status > :global(svg) {
    color: var(--accent);
    flex: none;
  }
  .status > span {
    flex: 1;
  }
  .muted {
    color: var(--muted);
  }
  .muted.error {
    color: var(--accent-2);
  }
  .small {
    height: 26px;
    padding: 0 10px;
    flex: none;
  }
  .modes {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .mode {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: 6px;
    font-size: 12.5px;
    cursor: pointer;
  }
  .mode:hover {
    background: var(--panel-2);
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .foot {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
</style>
