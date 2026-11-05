<script lang="ts">
  // An in-app message with buttons (showAlert in ui.svelte.ts): update offers,
  // what's new after an update, and errors.
  import { tick } from "svelte";
  import { closeAlert, ui } from "../lib/ui.svelte";
  import { renderMarkdown } from "../lib/markdown";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  const alert = $derived(ui.alert);
  const ICON = { update: "sparkles", success: "check", error: "alert" } as const;
  let box = $state<HTMLDivElement>();

  // The last button (the main action) takes the focus, so Enter picks it.
  $effect(() => {
    if (alert) tick().then(() => box?.querySelector<HTMLButtonElement>(".actions .btn:last-child")?.focus());
  });
</script>

<svelte:window onkeydown={(e) => alert && e.key === "Escape" && (e.preventDefault(), closeAlert(null))} />

{#if alert}
  <div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && closeAlert(null)}>
    <div class="dialog rise {alert.tone}" role="alertdialog" aria-modal="true" aria-labelledby="alert-title" aria-describedby="alert-message" bind:this={box}>
      <div class="head">
        <span class="badge"><Icon name={ICON[alert.tone]} size={16} /></span>
        <div>
          <h2 id="alert-title">{alert.title}</h2>
          <p id="alert-message">{alert.message}</p>
        </div>
      </div>
      {#if alert.markdown}
        <!-- Escaped first, then a fixed set of tags (markdown.ts). -->
        <div class="notes">{@html renderMarkdown(alert.markdown)}</div>
      {/if}
      <div class="actions">
        {#each alert.actions ?? [{ label: t("mcp.close"), value: "ok", primary: true }] as a (a.value)}
          <button class="btn" class:primary={a.primary} onclick={() => closeAlert(a.value)}>{a.label}</button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 130;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(0 0 0 / 0.45);
  }
  .dialog {
    width: min(520px, 100%);
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
  }
  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .badge {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }
  .success .badge {
    background: color-mix(in srgb, var(--ok) 16%, transparent);
    color: var(--ok);
  }
  .error .badge {
    background: color-mix(in srgb, var(--accent-2) 16%, transparent);
    color: var(--accent-2);
  }
  h2 {
    margin: 0;
    font: 500 17px var(--serif);
  }
  p {
    margin: 4px 0 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
    white-space: pre-wrap;
  }
  .notes {
    overflow: auto;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    font-size: 12.5px;
    line-height: 1.55;
    -webkit-user-select: text;
    user-select: text;
  }
  .notes :global(ul),
  .notes :global(ol) {
    margin: 0;
    padding-left: 18px;
  }
  .notes :global(li + li) {
    margin-top: 5px;
  }
  .notes :global(p) {
    margin: 0 0 8px;
    color: var(--text);
  }
  .notes :global(h4) {
    margin: 8px 0 6px;
    font-size: 12.5px;
  }
  .notes :global(code) {
    font: 11.5px var(--mono);
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--panel-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
