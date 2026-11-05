<script lang="ts">
  // Turning the MCP server on and adding it to the user's agent.
  import { onDestroy } from "svelte";
  import { ipc, type McpLink } from "../lib/ipc";
  import { AGENTS, agentById, clientName } from "../lib/agents";
  import { mcp, startMcp, stopMcp } from "../lib/mcp.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { copyText, ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  const agent = $derived(agentById(settings.agent));
  const setup = $derived(mcp.info ? agent.mcp(mcp.info) : null);
  const raw = $derived(mcp.info ? `URL: ${mcp.info.url}\nAuthorization: Bearer ${mcp.info.token}` : "");

  let link = $state<McpLink | null>(null);
  const poll = setInterval(async () => (link = settings.mcpEnabled ? await ipc.mcpLink().catch(() => null) : null), 1500);
  onDestroy(() => clearInterval(poll));

  const close = () => (ui.dialog = null);

  function pick(id: string) {
    settings.agent = id;
    persist();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="dialog rise" role="dialog" aria-modal="true" aria-labelledby="mcp-title">
    <header>
      <Icon name="plug" size={16} />
      <h2 id="mcp-title">{t("mcp.title")}</h2>
      <button class="icon-btn" onclick={close} title={t("mcp.close")} aria-label={t("mcp.close")}><Icon name="close" size={14} /></button>
    </header>

    <div class="body">
      <p class="note">{t("mcp.intro")}</p>
      <label class="toggle">
        <input type="checkbox" checked={settings.mcpEnabled} onchange={(e) => (e.currentTarget.checked ? startMcp() : stopMcp())} />
        {t("mcp.enable")}
      </label>
      {#if mcp.error}<p class="error">{mcp.error}</p>{/if}

      {#if settings.mcpEnabled && mcp.info && setup}
        <span class="label">{t("mcp.agent")}</span>
        <div class="agents" role="radiogroup" aria-label={t("mcp.agent")}>
          {#each AGENTS as a (a.id)}
            <button class="agent" class:on={a.id === agent.id} role="radio" aria-checked={a.id === agent.id} onclick={() => pick(a.id)}>{a.label}</button>
          {/each}
        </div>

        {#if setup.kind === "none"}
          <p class="note">{t("mcp.noSupport", { agent: agent.label })}</p>
          <div class="cmd">
            <code>{raw}</code>
            <button class="btn small" onclick={() => copyText(raw)}><Icon name="copy" size={12} />{t("mcp.copy")}</button>
          </div>
        {:else}
          <p class="note">{setup.kind === "command" ? t("mcp.stepCommand") : t("mcp.stepFile", { file: setup.file })}</p>
          <div class="cmd">
            <code>{setup.text}</code>
            <button class="btn small" onclick={() => copyText(setup.text)}><Icon name="copy" size={12} />{t("mcp.copy")}</button>
          </div>
          <details>
            <summary>{t("mcp.raw")}</summary>
            <div class="cmd">
              <code>{raw}</code>
              <button class="btn small" onclick={() => copyText(raw)}><Icon name="copy" size={12} />{t("mcp.copy")}</button>
            </div>
          </details>
        {/if}

        <p class="status" class:connected={link?.connected}>
          <span class="dot"></span>
          {link?.connected ? t("mcp.connected", { client: link.client ?? clientName(null) }) : t("mcp.waiting")}
        </p>
        <p class="note small">{t("mcp.safety")}</p>
      {/if}
    </div>
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
    width: min(620px, 100%);
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
  }
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
  .agents {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .agent {
    height: 26px;
    padding: 0 11px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--bg);
    color: var(--text);
    font-size: 12px;
  }
  .agent:hover {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .agent.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .cmd {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
  }
  .cmd code {
    flex: 1;
    min-width: 0;
    font: 11.5px/1.5 var(--mono);
    white-space: pre-wrap;
    word-break: break-all;
    -webkit-user-select: text;
    user-select: text;
  }
  .cmd .btn {
    flex: none;
  }
  details summary {
    cursor: pointer;
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 8px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .status.connected {
    color: var(--ok);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
  }
  .connected .dot {
    background: var(--ok);
    box-shadow: 0 0 6px color-mix(in srgb, var(--ok) 70%, transparent);
  }
</style>
