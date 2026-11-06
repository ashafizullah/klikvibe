<script lang="ts">
  // KlikVibe Remote: turning it on, adding a phone with a QR code, and the paired phones.
  import { onMount } from "svelte";
  import { ipc } from "../lib/ipc";
  import { refreshRemote, remote, remoteAction } from "../lib/remote.svelte";
  import { confirm, ui } from "../lib/ui.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import Dialog from "./Dialog.svelte";

  const LANDING = "https://klikvibe.ashafizullah.com";
  const s = $derived(remote.status);

  onMount(() => {
    refreshRemote();
  });

  function close() {
    // A QR code left on screen is no use once the dialog is gone (also one still being made).
    if (!s?.confirm) ipc.remotePairCancel().then((st) => (remote.status = st));
    ui.dialog = null;
  }

  async function unlink() {
    const ok = await confirm({ title: t("remote.unlink"), message: t("remote.unlinkConfirm"), confirm: t("remote.unlink"), danger: true });
    if (ok) remoteAction(() => ipc.remoteUnlink());
  }

  async function forget(id: string, name: string) {
    const ok = await confirm({ title: t("remote.forget"), message: t("remote.forgetConfirm", { name }), confirm: t("remote.forget"), danger: true });
    if (ok) remoteAction(() => ipc.remoteForgetPhone(id));
  }
</script>

<Dialog onclose={close} width={560} labelledby="remote-title">
  <header>
    <Icon name="phone" size={16} />
    <h2 id="remote-title">{t("remote.title")}</h2>
    <button class="icon-btn" onclick={close} title={t("remote.close")} aria-label={t("remote.close")}><Icon name="close" size={14} /></button>
  </header>

  <div class="body">
    <p class="note">{t("remote.intro")}</p>
    <label class="toggle">
      <input type="checkbox" checked={s?.enabled ?? false} disabled={remote.busy} onchange={(e) => remoteAction(() => ipc.remoteSetEnabled(e.currentTarget.checked))} />
      {t("remote.enable")}
    </label>
    {#if remote.error || s?.error}<p class="error">{remote.error ?? s?.error}</p>{/if}

    {#if s?.confirm}
      <div class="confirm" role="alertdialog" aria-labelledby="confirm-title">
        <Icon name="phone" size={22} />
        <div>
          <strong id="confirm-title">{t("remote.confirmTitle", { phone: s.confirm.phone })}</strong>
          <p>{s.confirm.account ? t("remote.confirmBody", { account: s.confirm.account }) : t("remote.confirmBodyNoAccount")}</p>
          <div class="actions">
            <button class="btn small" disabled={remote.busy} onclick={() => remoteAction(() => ipc.remotePairConfirm(false))}>{t("remote.deny")}</button>
            <button class="btn small primary" disabled={remote.busy} onclick={() => remoteAction(() => ipc.remotePairConfirm(true))}>{t("remote.allow")}</button>
          </div>
        </div>
      </div>
    {/if}
    {#if s?.enabled}
      {#if s.pairing}
        <div class="pair">
          <div class="qr" aria-label={t("remote.scanTitle")}>{@html s.pairing.svg}</div>
          <div class="pair-text">
            <strong>{t("remote.scanTitle")}</strong>
            <ol>
              <li>{t("remote.scan1")}</li>
              <li>{t("remote.scan2")}</li>
              <li>{t("remote.scan3")}</li>
            </ol>
            <button class="btn small" onclick={() => remoteAction(() => ipc.remotePairCancel())}>{t("common.cancel")}</button>
          </div>
        </div>
      {:else}
        <div class="row">
          <span class="label">{t("remote.phones")}</span>
          <button class="btn small primary" disabled={remote.busy} onclick={() => remoteAction(() => ipc.remotePairStart())}>
            <Icon name="plus" size={12} />{t("remote.addPhone")}
          </button>
        </div>
        {#if s.phones.length}
          <ul class="phones">
            {#each s.phones as p (p.id)}
              <li>
                <span class="dot" class:on={p.connected}></span>
                <span class="name ellipsis">{p.name}</span>
                <span class="state">{p.connected ? t("remote.connected") : t("remote.notConnected")}</span>
                <button class="icon-btn" onclick={() => forget(p.id, p.name)} title={t("remote.forget")} aria-label={t("remote.forget")}><Icon name="trash" size={13} /></button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="note">{t("remote.noPhones")}</p>
        {/if}
      {/if}

      <div class="facts">
        {#if s.linked}
          <span>{t("remote.account", { account: s.account ?? "…" })} · <b class:pro={s.pro}>{s.pro ? "Pro" : t("remote.free")}</b></span>
        {:else}
          <span>{t("remote.notLinked")}</span>
        {/if}
        {#if s.addresses.length && s.port}<span class="mono">{s.addresses.map((a) => `${a}:${s.port}`).join(" · ")}</span>{/if}
      </div>
      <p class="note small">{s.pro ? t("remote.anywhere") : t("remote.sameWifi")}</p>
      {#if s.pro && s.linked}
        <p class="status" class:connected={s.relay}><span class="dot" class:on={s.relay}></span>{s.relay ? t("remote.relayOn") : t("remote.relayOff")}</p>
      {/if}
      <p class="note small">{t("remote.safety")}</p>
      <div class="foot">
        <button class="link" onclick={() => openUrl(LANDING)}>{t("remote.getApp")}</button>
        {#if s.linked}<button class="link danger" onclick={unlink}>{t("remote.unlink")}</button>{/if}
      </div>
    {:else}
      <p class="note small">{t("remote.offNote")}</p>
      <button class="link" onclick={() => openUrl(LANDING)}>{t("remote.getApp")}</button>
    {/if}
  </div>
</Dialog>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 12px 12px 18px;
    border-bottom: 1px solid var(--line);
  }
  header :global(svg) {
    color: var(--accent);
  }
  h2 {
    flex: 1;
    margin: 0;
    font: 500 17px var(--serif);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px 18px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .note {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--muted);
  }
  .note.small {
    font-size: 11.5px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--accent-2);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
  }
  .phones {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .phones li {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px 0 12px;
    font-size: 12.5px;
  }
  .phones li + li {
    border-top: 1px solid var(--line);
  }
  .name {
    flex: 1;
    min-width: 0;
  }
  .state {
    font-size: 11.5px;
    color: var(--muted);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
    flex: none;
  }
  .dot.on {
    background: var(--ok);
    box-shadow: 0 0 6px color-mix(in srgb, var(--ok) 70%, transparent);
  }
  .pair {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg);
  }
  .qr {
    flex: none;
    width: 180px;
    height: 180px;
    padding: 8px;
    border-radius: 8px;
    background: #fff;
  }
  .qr :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .pair-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    font-size: 12.5px;
  }
  .pair-text ol {
    margin: 0;
    padding-left: 18px;
    color: var(--muted);
    line-height: 1.6;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .status.connected {
    color: var(--ok);
  }
  .confirm {
    display: flex;
    gap: 14px;
    padding: 14px 16px;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 9%, transparent);
  }
  .confirm :global(svg) {
    flex: none;
    color: var(--accent);
    margin-top: 2px;
  }
  .confirm p {
    margin: 4px 0 12px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .confirm .actions {
    display: flex;
    gap: 8px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 6px 12px;
    font-size: 12px;
  }
  .facts b.pro {
    color: var(--accent);
  }
  .mono {
    font: 11.5px var(--mono);
    color: var(--muted);
  }
  .foot {
    display: flex;
    justify-content: space-between;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12px;
  }
  .link.danger {
    color: var(--danger);
  }
</style>
