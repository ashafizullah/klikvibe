<script lang="ts">
  // An in-app message with buttons (showAlert in ui.svelte.ts): update offers,
  // what's new after an update, and errors.
  import { tick } from "svelte";
  import { closeAlert, ui } from "../lib/ui.svelte";
  import { renderMarkdown } from "../lib/markdown";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import Dialog from "./Dialog.svelte";

  const alert = $derived(ui.alert);
  const ICON = { update: "sparkles", success: "check", error: "alert", warning: "alert", danger: "alert" } as const;
  let box = $state<HTMLDivElement>();
  let text = $state("");
  $effect(() => {
    text = alert?.input?.value ?? "";
  });

  /** With a text field, the main action answers with the text. */
  function choose(value: string) {
    closeAlert(alert?.input && value === "\u0000ok" ? text : value);
  }

  // The main action (the last button) takes the focus so Enter picks it; when it destroys something, Cancel does.
  $effect(() => {
    if (!alert) return;
    const safe = alert.actions?.some((a) => a.danger);
    tick().then(() =>
      (alert?.input ? box?.querySelector<HTMLElement>("input.answer") : box?.querySelector<HTMLButtonElement>(safe ? ".actions .btn:first-child" : ".actions .btn:last-child"))?.focus(),
    );
  });
</script>

{#if alert}
  <Dialog onclose={() => closeAlert(null)} width={520} padding={20} gap={14} alert labelledby="alert-title" describedby="alert-message" bind:panel={box}>
    <div class="tone {alert.tone}">
      <div class="head">
        <span class="badge"><Icon name={ICON[alert.tone]} size={16} /></span>
        <div>
          <h2 id="alert-title">{alert.title}</h2>
          <p id="alert-message">{alert.message}</p>
        </div>
      </div>
      {#if alert.input}
        <input
          class="field answer"
          bind:value={text}
          placeholder={alert.input.placeholder}
          spellcheck="false"
          onkeydown={(e) => e.key === "Enter" && (e.preventDefault(), choose("\u0000ok"))}
        />
      {/if}
      {#if alert.markdown}
        <!-- Escaped first, then a fixed set of tags (markdown.ts). -->
        <div class="notes">{@html renderMarkdown(alert.markdown)}</div>
      {/if}
      <div class="actions">
        {#each alert.actions ?? [{ label: t("mcp.close"), value: "ok", primary: true }] as a (a.value)}
          <button class="btn" class:primary={a.primary} class:danger={a.danger} onclick={() => choose(a.value)}>{a.label}</button>
        {/each}
      </div>
    </div>
  </Dialog>
{/if}

<style>
  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  /* Lets the head, notes and buttons sit directly in the dialog's column. */
  .tone {
    display: contents;
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
  .warning .badge {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }
  .danger .badge {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
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
