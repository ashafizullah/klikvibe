<script lang="ts">
  // Shells in the project folder: one for the agent, more for a dev server or tests.
  import { useProject } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { isLastPanel, togglePanel } from "../lib/layout";
  import { AGENTS, agentById } from "../lib/agents";
  import { t } from "../lib/i18n/index.svelte";
  import { openMenu, openMenuBelow, ui, type MenuItem } from "../lib/ui.svelte";
  import { onDestroy, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import TerminalSession from "./TerminalSession.svelte";

  /** `fill`: the terminal takes its whole area (a column) instead of a resizable height under the code. */
  let { shown, fill }: { shown: boolean; fill: boolean } = $props();
  const project = useProject();

  interface Shell {
    id: number;
    exited: boolean;
    /** The user's name for it; it wins over the automatic one. */
    name: string | null;
    /** From what runs in it (terminalName in agents.ts); null for the bare shell. */
    auto: string | null;
  }

  let nextId = 1;
  const shell = (): Shell => ({ id: nextId++, exited: false, name: null, auto: null });
  let shells = $state<Shell[]>([shell()]);
  let active = $state(1);
  const refs = $state<Record<number, TerminalSession>>({});
  const cwd = project.root;
  const agent = $derived(agentById(settings.agent));

  function add() {
    const s = shell();
    shells.push(s);
    active = s.id;
  }

  const label = (s: Shell, i: number) => s.name ?? s.auto ?? t("terminal.shell", { n: i + 1 });

  // Renaming in place: the tab turns into a text field.
  let renaming = $state<number | null>(null);
  let draft = $state("");
  let field = $state<HTMLInputElement>();

  async function rename(s: Shell, i: number) {
    renaming = s.id;
    draft = label(s, i);
    await tick();
    field?.focus();
    field?.select();
  }

  function finishRename(save: boolean) {
    const s = shells.find((x) => x.id === renaming);
    // An empty name goes back to the automatic one.
    if (s && save) s.name = draft.trim() || null;
    renaming = null;
  }

  function shellMenu(e: MouseEvent, s: Shell, i: number) {
    openMenu(e, [
      { label: t("terminal.rename"), run: () => rename(s, i) },
      { label: t("terminal.resetName"), disabled: s.name === null, run: () => (s.name = null) },
      "sep",
      { label: t("terminal.new"), run: add },
      { label: t("terminal.close"), disabled: shells.length < 2, run: () => close(s.id) },
    ]);
  }

  function close(id: number) {
    const i = shells.findIndex((s) => s.id === id);
    shells.splice(i, 1);
    delete refs[id];
    if (!shells.length) add();
    else if (active === id) active = shells[Math.max(0, i - 1)].id;
  }

  function runAgent() {
    // Only types the command: the user presses Enter and runs the agent themselves.
    refs[active]?.type(agent.command);
  }

  const isAgent = (s: Shell) => s.auto !== null && AGENTS.some((a) => a.label === s.auto);

  /** Pastes into the terminal that runs an agent (the active one first), else the active terminal. */
  function askAgent(text: string) {
    const target = shells.find((s) => s.id === active && isAgent(s)) ?? shells.find(isAgent) ?? shells.find((s) => s.id === active);
    if (!target) return;
    active = target.id;
    if (!settings.showTerminal) {
      settings.showTerminal = true;
      persist();
    }
    // The panel may have just been shown.
    tick().then(() => refs[target.id]?.paste(text));
  }
  project.askAgent = askAgent;
  const terminals = () =>
    shells.map((s, i) => ({ index: i + 1, name: label(s, i), active: s.id === active, agent: isAgent(s), tail: (n: number) => refs[s.id]?.tail(n) ?? "" }));
  project.terminals = terminals;
  onDestroy(() => {
    if (project.askAgent === askAgent) project.askAgent = null;
    if (project.terminals === terminals) project.terminals = null;
  });

  const BUILT_IN = ["prompts.fixTests", "prompts.review", "prompts.explain", "prompts.commitMessage", "prompts.simpler"] as const;

  function promptsMenu(e: MouseEvent) {
    const items: MenuItem[] = [
      ...BUILT_IN.map((k) => ({ label: t(k), run: () => askAgent(t(k)) })),
      ...(settings.prompts.length ? ["sep" as const] : []),
      ...settings.prompts.map((p) => ({ label: p, run: () => askAgent(p) })),
      "sep",
      { label: t("prompts.edit"), run: () => (ui.dialog = "prompts") },
    ];
    openMenuBelow(e.currentTarget as Element, items);
  }

  function setAgent(e: Event) {
    settings.agent = (e.currentTarget as HTMLSelectElement).value;
    persist();
  }

  function startDrag(e: PointerEvent) {
    const startY = e.clientY;
    const start = settings.terminalHeight;
    const move = (ev: PointerEvent) => (settings.terminalHeight = Math.min(window.innerHeight * 0.75, Math.max(120, start + startY - ev.clientY)));
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      persist();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
</script>

{#if !settings.showTerminal && !fill}
  <!-- Hidden terminals keep running; this strip brings them back. -->
  <button class="collapsed" onclick={() => ((settings.showTerminal = true), persist())} title={`${t("terminal.show")} (Ctrl+\`)`}>
    <Icon name="terminal" size={13} />
    <span>{t("terminal.title")}</span>
    {#if shells.length > 1}<span class="count-badge">{shells.length}</span>{/if}
    <span class="spacer"></span>
    <span class="kbd">Ctrl+`</span>
    <Icon name="up" size={13} />
  </button>
{/if}

<section class="terminal" class:hidden={!settings.showTerminal} class:fill style:height={fill ? null : `${settings.terminalHeight}px`} aria-label={t("terminal.title")}>
  {#if !fill}<div class="grip" role="separator" aria-orientation="horizontal" onpointerdown={startDrag}></div>{/if}
  <div class="head">
    <div class="shells">
      {#each shells as s, i (s.id)}
        <div class="shell" class:on={s.id === active} class:exited={s.exited}>
          {#if renaming === s.id}
            <input
              class="shell-name"
              bind:this={field}
              bind:value={draft}
              onkeydown={(e) => (e.key === "Enter" ? finishRename(true) : e.key === "Escape" ? finishRename(false) : null)}
              onblur={() => finishRename(true)}
              aria-label={t("terminal.rename")}
              spellcheck="false"
            />
          {:else}
            <button
              class="shell-main ellipsis"
              onclick={() => (active = s.id)}
              ondblclick={() => rename(s, i)}
              oncontextmenu={(e) => shellMenu(e, s, i)}
              title={`${label(s, i)} · ${t("terminal.renameHint")}`}
            >
              {label(s, i)}
            </button>
          {/if}
          {#if shells.length > 1}
            <button class="shell-close" onclick={() => close(s.id)} aria-label={t("terminal.close")} title={t("terminal.close")}><Icon name="close" size={10} /></button>
          {/if}
        </div>
      {/each}
      <button class="icon-btn add" onclick={add} title={t("terminal.new")} aria-label={t("terminal.new")}><Icon name="plus" size={13} /></button>
    </div>
    <div class="spacer"></div>
    <div class="agent">
      <button class="btn small" onclick={promptsMenu} title={t("prompts.hint")}>
        <Icon name="message-square-text" size={13} />{t("prompts.title")}
      </button>
      <select class="field" value={settings.agent} onchange={setAgent} aria-label={t("terminal.agent")}>
        {#each AGENTS as a (a.id)}<option value={a.id}>{a.label}</option>{/each}
      </select>
      <button class="btn primary small" onclick={runAgent} title={t("terminal.runAgentHint", { command: agent.command })}>
        <Icon name="sparkles" size={13} />{t("terminal.runAgent")}
      </button>
    </div>
    <button
      class="icon-btn"
      class:on={settings.notifyAgents}
      onclick={() => ((settings.notifyAgents = !settings.notifyAgents), persist())}
      title={settings.notifyAgents ? t("notify.turnOff") : t("notify.turnOn")}
      aria-label={settings.notifyAgents ? t("notify.turnOff") : t("notify.turnOn")}
      aria-pressed={settings.notifyAgents}
    >
      <Icon name={settings.notifyAgents ? "bell" : "bell-off"} size={14} />
    </button>
    <button class="icon-btn" onclick={() => togglePanel("showTerminal")} disabled={isLastPanel("showTerminal")} title={t("terminal.hide")} aria-label={t("terminal.hide")}>
      <Icon name={fill ? "close" : "down"} size={fill ? 14 : 16} />
    </button>
  </div>
  <div class="body">
    {#if cwd}
      {#each shells as s (s.id)}
        <TerminalSession bind:this={refs[s.id]} onname={(n) => (s.auto = n)} {cwd} visible={s.id === active && settings.showTerminal && shown} onexit={() => (s.exited = true)} />
      {/each}
    {/if}
  </div>
</section>

<style>
  .terminal {
    position: relative;
    z-index: 1;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-top: 1px solid var(--line);
  }
  .terminal.hidden {
    display: none;
  }
  .terminal.fill {
    flex: 1;
    min-height: 0;
    border-top: none;
  }
  .collapsed {
    height: 28px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px 0 12px;
    border: none;
    border-top: 1px solid var(--line);
    border-radius: 0;
    background: var(--panel);
    color: var(--muted);
    font-size: 12px;
    text-align: left;
  }
  .collapsed:hover {
    color: var(--text);
    background: var(--panel-2);
  }
  .collapsed:hover :global(svg) {
    color: var(--accent);
  }
  .grip {
    position: absolute;
    left: 0;
    right: 0;
    top: -4px;
    height: 8px;
    cursor: row-resize;
    z-index: 2;
  }
  .head {
    height: 36px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 6px 0 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .shells {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .shell {
    display: flex;
    align-items: center;
    height: 22px;
    border-radius: 5px;
    color: var(--muted);
  }
  .shell.on {
    background: var(--panel-2);
    color: var(--accent);
  }
  .shell.exited {
    opacity: 0.55;
  }
  .shell-main {
    max-width: 150px;
    border: none;
    background: none;
    font: 11px var(--mono);
    padding: 0 7px;
    height: 100%;
  }
  .shell-name {
    width: 130px;
    height: 20px;
    margin: 0 2px;
    padding: 0 6px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    outline: none;
    background: var(--bg);
    color: var(--text);
    font: 11px var(--mono);
    -webkit-user-select: text;
    user-select: text;
  }
  .shell-close {
    border: none;
    background: none;
    padding: 0 4px 0 0;
    color: var(--muted);
    display: flex;
  }
  .add {
    width: 22px;
    height: 22px;
  }
  .spacer {
    flex: 1;
  }
  .agent {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .agent select {
    height: 24px;
    font-size: 11.5px;
  }
  .small {
    height: 24px;
    padding: 0 9px;
  }
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
  }
</style>
