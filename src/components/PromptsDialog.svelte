<script lang="ts">
  // The user's own prompts for the terminal's Prompts menu.
  import { persist, settings } from "../lib/settings.svelte";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import Dialog from "./Dialog.svelte";

  let draft = $state("");
  const close = () => (ui.dialog = null);

  function add() {
    const p = draft.trim();
    if (!p || settings.prompts.includes(p)) return;
    settings.prompts = [...settings.prompts, p];
    persist();
    draft = "";
  }

  function remove(i: number) {
    settings.prompts = settings.prompts.filter((_, k) => k !== i);
    persist();
  }
</script>

<Dialog onclose={close} width={520} padding={20} gap={14} labelledby="prompts-title">
  <div class="head">
    <Icon name="message-square-text" size={16} />
    <h2 id="prompts-title">{t("prompts.dialogTitle")}</h2>
    <button class="icon-btn" onclick={close} title={t("mcp.close")} aria-label={t("mcp.close")}><Icon name="close" size={14} /></button>
  </div>
  <p class="note">{t("prompts.intro")}</p>
  <form class="add" onsubmit={(e) => (e.preventDefault(), add())}>
    <textarea
      class="field"
      rows="2"
      bind:value={draft}
      placeholder={t("prompts.placeholder")}
      onkeydown={(e) => (e.metaKey || e.ctrlKey) && e.key === "Enter" && (e.preventDefault(), add())}
    ></textarea>
    <button class="btn primary" type="submit" disabled={!draft.trim()}><Icon name="plus" size={13} />{t("prompts.add")}</button>
  </form>
  {#if settings.prompts.length}
    <ul class="list">
      {#each settings.prompts as p, i (p)}
        <li>
          <span class="text">{p}</span>
          <button class="icon-btn" onclick={() => remove(i)} title={t("prompts.remove")} aria-label={t("prompts.remove")}><Icon name="close" size={12} /></button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="note">{t("prompts.none")}</p>
  {/if}
</Dialog>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .head :global(svg) {
    color: var(--accent);
  }
  h2 {
    flex: 1;
    margin: 0;
    font: 500 17px var(--serif);
  }
  .note {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .add {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .add textarea {
    flex: 1;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .list li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 4px 6px 10px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--bg);
  }
  .text {
    flex: 1;
    min-width: 0;
    padding-top: 5px;
    font-size: 12.5px;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
