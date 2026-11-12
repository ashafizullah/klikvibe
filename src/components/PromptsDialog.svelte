<script lang="ts">
  // The terminal's Prompts menu: add a prompt, edit or remove any, the built-in ones included.
  import { missingBuiltIns, promptList, restoreBuiltIns, savePrompts } from "../lib/prompts.svelte";
  import { ui } from "../lib/ui.svelte";
  import { onDestroy } from "svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import Dialog from "./Dialog.svelte";

  let list = $state(promptList());
  let draft = $state("");
  const close = () => (ui.dialog = null);
  const save = () => savePrompts($state.snapshot(list));
  const missing = $derived(missingBuiltIns());

  /** Tall enough for its text, up to six lines. */
  const rows = (p: string) => Math.min(6, Math.max(1, p.split("\n").reduce((n, line) => n + Math.max(1, Math.ceil(line.length / 64)), 0)));

  function add() {
    const p = draft.trim();
    if (!p || list.includes(p)) return;
    list.push(p);
    save();
    draft = "";
  }

  function remove(i: number) {
    list.splice(i, 1);
    save();
  }

  // A prompt emptied by an edit is dropped when the dialog closes.
  onDestroy(() => {
    if (list.some((p) => !p.trim())) savePrompts($state.snapshot(list).filter((p) => p.trim()));
  });

  function restore() {
    restoreBuiltIns();
    list = promptList();
  }
</script>

<Dialog onclose={close} width={560} padding={20} gap={14} labelledby="prompts-title">
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
  {#if list.length}
    <ul class="list">
      {#each list as _, i (i)}
        <li>
          <textarea class="text" rows={rows(list[i])} bind:value={list[i]} oninput={save} aria-label={t("prompts.editOne")} spellcheck="false"></textarea>
          <button class="icon-btn" onclick={() => remove(i)} title={t("prompts.remove")} aria-label={t("prompts.remove")}><Icon name="close" size={12} /></button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="note">{t("prompts.none")}</p>
  {/if}
  {#if missing.length}
    <div class="foot">
      <button class="btn ghost small" onclick={restore}><Icon name="rotate-ccw" size={12} />{t("prompts.restore")}</button>
    </div>
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
    max-height: 46vh;
    overflow-y: auto;
  }
  .list li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 2px 4px 2px 4px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--bg);
  }
  .list li:focus-within {
    border-color: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
    padding: 6px;
    border: none;
    outline: none;
    resize: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    line-height: 1.45;
    -webkit-user-select: text;
    user-select: text;
  }
  .list .icon-btn {
    margin-top: 2px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
  }
</style>
