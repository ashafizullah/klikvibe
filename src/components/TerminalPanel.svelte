<script lang="ts">
  // Shells in the project folder: one for the agent, more for a dev server or tests.
  import { useProject } from "../lib/project.svelte";
  import { persist, settings } from "../lib/settings.svelte";
  import { isLastPanel, togglePanel } from "../lib/layout";
  import { AGENTS, type AgentDef } from "../lib/agents";
  import { sessionHeld } from "../lib/session.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import { openMenu, openMenuBelow, toast, ui, type MenuItem } from "../lib/ui.svelte";
  import { CARGO_SCRIPTS, packageManager, packageScripts, type Script } from "../lib/scripts";
  import { ipc } from "../lib/ipc";
  import { join } from "../lib/paths";
  import { onDestroy, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import TerminalSession from "./TerminalSession.svelte";
  import { promptList } from "../lib/prompts.svelte";
  import ScrollButton from "./ScrollButton.svelte";
  import { StripScroll } from "../lib/stripScroll.svelte";

  /** `fill`: the terminal takes its whole area (a column) instead of a resizable height under the code. */
  let { shown, fill }: { shown: boolean; fill: boolean } = $props();
  const project = useProject();
  const strip = new StripScroll();

  interface Shell {
    id: number;
    exited: boolean;
    /** The user's name for it; it wins over the automatic one. */
    name: string | null;
    /** From what runs in it (terminalName in agents.ts); null for the bare shell. */
    auto: string | null;
    /** A dev server's address it printed, for the preview. */
    url: string | null;
    /** A script to run once it starts. */
    run?: string;
    /** Typed at the prompt once it starts, without Enter. */
    prefill?: string;
    /** Its output from before KlikVibe was closed. */
    history?: string;
  }

  let nextId = 1;
  const shell = (run?: string): Shell => ({ id: nextId++, exited: false, name: null, auto: null, url: null, run });

  // ---- the terminals as they were when KlikVibe was closed ----

  /** Saved per project: the terminals, their names, their last output and the agent each ran. */
  interface SavedShell {
    name: string | null;
    agent: string | null;
    history: string;
  }
  const SAVE_KEY = `klikvibe.terminals:${project.root}`;
  /** Lines of output kept per terminal. */
  const HISTORY_LINES = 300;

  function restored(): { shells: Shell[]; active: number } | null {
    let saved: { shells: SavedShell[]; active: number } | null = null;
    try {
      saved = JSON.parse(localStorage.getItem(SAVE_KEY) ?? "null");
    } catch {}
    if (!saved?.shells?.length) return null;
    const list = saved.shells.map((x) => {
      const s = shell();
      s.name = x.name;
      s.history = x.history || undefined;
      // An agent that was running starts again in its last conversation; one
      // that can't resume is only typed, for the user to start.
      const a = x.agent ? AGENTS.find((d) => d.id === x.agent) : undefined;
      if (a?.resume) s.run = a.resume;
      else if (a) s.prefill = a.command;
      return s;
    });
    return { shells: list, active: list[Math.min(Math.max(0, saved.active), list.length - 1)].id };
  }

  const start = restored();
  let shells = $state<Shell[]>(start?.shells ?? [shell()]);
  let active = $state(start?.active ?? 1);
  const refs = $state<Record<number, TerminalSession>>({});
  const cwd = project.root;
  /** `auto`: its name until one is read from what runs in it. */
  function add(run?: string, auto: string | null = null) {
    const s = shell(run);
    s.auto = auto;
    shells.push(s);
    active = s.id;
    return s;
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
      { label: t("terminal.new"), run: () => add() },
      { label: t("terminal.close"), run: () => close(s.id) },
    ]);
  }

  function close(id: number) {
    const i = shells.findIndex((s) => s.id === id);
    shells.splice(i, 1);
    delete refs[id];
    if (!shells.length) add();
    else if (active === id) active = shells[Math.max(0, i - 1)].id;
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
  project.remoteTerminals = () =>
    shells.flatMap((s, i) => {
      const id = refs[s.id]?.sessionId();
      // An ended shell stays listed (as "exited"), as its tab stays here: the phone can read its last output and close it.
      if (!id) return [];
      const agent = isAgent(s) ? (AGENTS.find((a) => a.label === s.auto)?.id ?? null) : null;
      return [{ id, name: label(s, i), agent }];
    });

  /** How long a new shell may take to start before the phone is told it failed. */
  const START_WAIT = 8000;
  /**
   * A terminal the phone asked for (KlikVibe Remote's `term.new`). The panel of every open
   * project stays mounted (ProjectView), and a shell starts even while the panel is hidden,
   * so nothing has to be shown first.
   */
  async function newTerminal(a: AgentDef | null): Promise<string> {
    // Named after its agent at once, so the phone shows which it is before the name is read from the shell.
    const s = add(a?.command, a?.label ?? null);
    const until = Date.now() + START_WAIT;
    while (Date.now() < until && shells.some((x) => x.id === s.id)) {
      const id = refs[s.id]?.sessionId();
      if (id) return id;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error("The terminal did not start");
  }
  project.newTerminal = newTerminal;

  /** A terminal the phone closed (`term.kill`): like closing its tab here. */
  function closeTerminal(session: string): boolean {
    const s = shells.find((x) => refs[x.id]?.sessionId() === session);
    if (!s) return false;
    close(s.id);
    return true;
  }
  project.closeTerminal = closeTerminal;

  /** A terminal's history as the window shows it, for a phone opening it; null if it isn't here. */
  function snapshot(session: string, lines: number) {
    const s = shells.find((x) => refs[x.id]?.sessionId() === session);
    return s ? (refs[s.id]?.snapshot(lines) ?? null) : null;
  }
  project.terminalSnapshot = snapshot;

  // Kept up to date while it runs: KlikVibe can be quit (or the computer shut down) at any moment.
  let lastSaved = "";
  function save() {
    const state = {
      active: shells.findIndex((s) => s.id === active),
      shells: shells.map((s): SavedShell => ({
        name: s.name,
        agent: isAgent(s) ? (AGENTS.find((a) => a.label === s.auto)?.id ?? null) : null,
        history: refs[s.id]?.tail(HISTORY_LINES) ?? s.history ?? "",
      })),
    };
    const json = JSON.stringify(state);
    if (json === lastSaved) return;
    lastSaved = json;
    try {
      localStorage.setItem(SAVE_KEY, json);
    } catch {}
  }
  const saver = setInterval(save, 3000);
  const saveNow = () => save();
  window.addEventListener("pagehide", saveNow);

  onDestroy(() => {
    clearInterval(saver);
    window.removeEventListener("pagehide", saveNow);
    if (project.askAgent === askAgent) project.askAgent = null;
    if (project.terminals === terminals) project.terminals = null;
    if (project.newTerminal === newTerminal) project.newTerminal = null;
    if (project.closeTerminal === closeTerminal) project.closeTerminal = null;
    if (project.terminalSnapshot === snapshot) project.terminalSnapshot = null;
    // The project was closed: its terminals don't come back next time. (An update keeps them.)
    if (sessionHeld()) save();
    else
      try {
        localStorage.removeItem(SAVE_KEY);
      } catch {}
  });

  // ---- scripts and the preview ----

  async function scriptsMenu(e: MouseEvent) {
    const el = e.currentTarget as Element;
    const names = (project.tree.get(project.root) ?? []).map((x) => x.name);
    const scripts: Script[] = [];
    if (names.includes("package.json")) {
      const pkg = await ipc.readFile(join(project.root, "package.json")).catch(() => null);
      scripts.push(...packageScripts(pkg?.text ?? "", packageManager(names)));
    }
    if (names.includes("Cargo.toml")) scripts.push(...CARGO_SCRIPTS);
    const items: MenuItem[] = scripts.length
      ? scripts.map((s) => ({ label: s.name, hint: s.detail && s.detail.length > 28 ? `${s.detail.slice(0, 27)}…` : s.detail, icon: "play", run: () => add(s.command) }))
      : [{ label: t("scripts.none"), disabled: true, run: () => {} }];
    openMenuBelow(el, items);
  }

  /** A dev server came up in a terminal: offer the preview, once per address. */
  const offered = new Set<string>();
  function foundUrl(s: Shell, url: string) {
    s.url = url;
    if (offered.has(url)) return;
    offered.add(url);
    toast(t("preview.found", { url }), "ok", { label: t("preview.open"), run: () => project.openPreview(url) }, 10_000);
  }

  function promptsMenu(e: MouseEvent) {
    const list = promptList().filter((p) => p.trim());
    const items: MenuItem[] = [
      ...list.map((p) => ({ label: p, run: () => askAgent(p) })),
      ...(list.length ? ["sep" as const] : []),
      { label: t("prompts.edit"), run: () => (ui.dialog = "prompts") },
    ];
    openMenuBelow(e.currentTarget as Element, items);
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
    <div class="tabs">
      {#if strip.overflow}<ScrollButton {strip} dir={-1} />{/if}
      <div class="shells" bind:this={strip.el}>
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
            {#if s.url}
              <button class="shell-web" onclick={() => project.openPreview(s.url!)} title={`${t("preview.open")} · ${s.url}`} aria-label={t("preview.open")}><Icon name="globe" size={11} /></button>
            {/if}
            <button class="shell-close" onclick={() => close(s.id)} aria-label={t("terminal.close")} title={t("terminal.close")}><Icon name="close" size={10} /></button>
          </div>
        {/each}
      </div>
      {#if strip.overflow}<ScrollButton {strip} dir={1} />{/if}
      <button class="icon-btn add" onclick={() => add()} title={t("terminal.new")} aria-label={t("terminal.new")}><Icon name="plus" size={13} /></button>
    </div>
    <div class="spacer"></div>
    <div class="agent">
      <button class="btn small" onclick={scriptsMenu} title={t("scripts.hint")}>
        <Icon name="play" size={13} />{t("scripts.title")}
      </button>
      <button class="btn small" onclick={promptsMenu} title={t("prompts.hint")}>
        <Icon name="message-square-text" size={13} />{t("prompts.title")}
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
        <TerminalSession bind:this={refs[s.id]} onname={(n) => (s.auto = n)} onurl={(u) => foundUrl(s, u)} run={s.run} prefill={s.prefill} history={s.history} {cwd} visible={s.id === active && settings.showTerminal && shown} onexit={() => (s.exited = true)} />
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
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .shells {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .shell {
    flex: none;
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
  .shell-web {
    display: flex;
    padding: 0 4px 0 0;
    border: none;
    background: none;
    color: var(--accent);
  }
  /* On the tab under the pointer and the open one, as on the project and file tabs. */
  .shell-close {
    border: none;
    background: none;
    padding: 0 4px 0 0;
    color: var(--muted);
    display: flex;
    opacity: 0;
  }
  .shell:hover .shell-close,
  .shell.on .shell-close,
  .shell-close:focus-visible {
    opacity: 1;
  }
  .shell-close:hover {
    color: var(--text);
  }
  .add {
    width: 22px;
    height: 22px;
  }
  .spacer {
    flex: 1;
  }
  .agent {
    flex: none;
    display: flex;
    gap: 6px;
    align-items: center;
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
