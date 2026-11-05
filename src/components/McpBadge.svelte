<script lang="ts">
  // Title-bar status of the MCP link: green once an agent is connected.
  import { onDestroy } from "svelte";
  import { ipc, type McpLink } from "../lib/ipc";
  import { clientName } from "../lib/agents";
  import { settings } from "../lib/settings.svelte";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";

  let link = $state<McpLink | null>(null);
  const poll = setInterval(async () => (link = settings.mcpEnabled ? await ipc.mcpLink().catch(() => null) : null), 2000);
  onDestroy(() => clearInterval(poll));

  const title = $derived(link?.connected ? t("mcp.badgeConnectedTitle", { client: link.client ?? clientName(null) }) : t("mcp.badgeWaitingTitle"));
</script>

{#if link?.running}
  <button class="badge" class:connected={link.connected} {title} aria-label={title} onclick={() => (ui.dialog = "mcp")}>
    <span class="dot"></span>
    {link.connected ? clientName(link.client) : t("mcp.badgeOn")}
  </button>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border: 1px solid color-mix(in srgb, var(--ok) 30%, transparent);
    border-radius: 999px;
    background: none;
    color: color-mix(in srgb, var(--ok) 80%, transparent);
    font-size: 11px;
    flex: none;
  }
  .badge:hover {
    color: var(--ok);
  }
  .badge.connected {
    border-color: color-mix(in srgb, var(--ok) 50%, transparent);
    background: color-mix(in srgb, var(--ok) 15%, transparent);
    color: var(--ok);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .connected .dot {
    box-shadow: 0 0 6px color-mix(in srgb, var(--ok) 70%, transparent);
  }
</style>
