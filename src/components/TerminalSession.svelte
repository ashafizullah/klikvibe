<script lang="ts">
  // One shell in the project folder. The user drives everything typed into it.
  import { onDestroy, onMount } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { errorText, ipc } from "../lib/ipc";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { useProject } from "../lib/project.svelte";
  import { findLinks, projectPath } from "../lib/termLinks";
  import { needsYou } from "../lib/attention.svelte";
  import { agentIn } from "../lib/agents";
  import { join } from "../lib/paths";
  import { toast } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";

  let { cwd, visible, onexit }: { cwd: string; visible: boolean; onexit: (code: number | null) => void } = $props();
  const project = useProject();

  // ---- an agent waiting for the user ----

  /** Output after this much silence starts a new burst of work. */
  const QUIET = 6000;
  /** A burst shorter than this (a command's output, typing) isn't an agent at work. */
  const BUSY = 3000;
  const key = `${project.root}#${Math.random().toString(36).slice(2)}`;
  let lastInput = 0;
  let lastOutput = 0;
  let burstStart = 0;
  let burstNotified = true;

  function noteOutput() {
    const now = Date.now();
    // The echo of what the user types isn't the agent working.
    if (now - lastInput < 1000) return;
    if (now - lastOutput > QUIET) {
      burstStart = now;
      burstNotified = false;
    }
    lastOutput = now;
  }

  async function tell(agent: string | null, message: string) {
    const name = agent ?? agentIn(session ? await ipc.terminalForeground(session).catch(() => null) : null) ?? t("terminal.title");
    needsYou(key, project.root, `${name} · ${project.current.name}`, message, visible && document.hasFocus());
  }

  /** The bell, agents' own notifications (OSC 9 and 777), and an agent going quiet after working. */
  function watchForAgent(xterm: Terminal) {
    const bell = xterm.onBell(() => tell(null, t("notify.bell")));
    // OSC 9 "message" (iTerm2); "9;4;…" is a progress report, not a message.
    const osc9 = xterm.parser.registerOscHandler(9, (data) => {
      if (/^\d+;/.test(data)) return false;
      tell(null, data);
      return true;
    });
    // OSC 777 "notify;title;body".
    const osc777 = xterm.parser.registerOscHandler(777, (data) => {
      const [cmd, title, ...body] = data.split(";");
      if (cmd !== "notify") return false;
      tell(null, body.join(";") || title || t("notify.bell"));
      return true;
    });
    const timer = setInterval(async () => {
      const now = Date.now();
      if (burstNotified || !session || now - lastOutput < QUIET || lastOutput - burstStart < BUSY) return;
      burstNotified = true;
      const agent = agentIn(await ipc.terminalForeground(session).catch(() => null));
      if (agent) tell(agent, t("notify.waiting"));
    }, 2000);
    return () => {
      bell.dispose();
      osc9.dispose();
      osc777.dispose();
      clearInterval(timer);
    };
  }

  /** File paths (that exist in the project) and web addresses in the output, as links. */
  function links(xterm: Terminal) {
    return xterm.registerLinkProvider({
      provideLinks(y, callback) {
        const text = xterm.buffer.active.getLine(y - 1)?.translateToString(true) ?? "";
        const found = findLinks(text).flatMap((l) => {
          let activate: () => void;
          if (l.kind === "url") activate = () => openUrl(l.target).catch((e) => toast(errorText(e), "error"));
          else {
            const rel = projectPath(l.target, project.root);
            if (!rel || !project.hasFile(rel)) return [];
            const abs = join(project.root, rel);
            activate = () => project.openFile(abs).then(() => l.line && project.revealLines(`file:${abs}`, l.line, l.line));
          }
          return [{ range: { start: { x: l.start + 1, y }, end: { x: l.end, y } }, text: text.slice(l.start, l.end), activate, decorations: { underline: true, pointerCursor: true } }];
        });
        callback(found.length ? found : undefined);
      },
    });
  }

  const decode = (b64: string) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));

  let host: HTMLDivElement;
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let session: string | null = null;
  let exited = $state(false);
  let cleanup: (() => void) | null = null;

  onMount(() => {
    const xterm = new Terminal({
      fontFamily: '"IBM Plex Mono", "SF Mono", Menlo, Consolas, monospace',
      fontSize: 12.5,
      cursorBlink: true,
      allowProposedApi: true,
      scrollback: 5000,
      theme: { background: "#15130f", foreground: "#ede6d8", cursor: "#f0a83a", cursorAccent: "#15130f", selectionBackground: "#f0a83a44" },
    });
    const fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);
    xterm.open(host);
    // Let the app shortcuts (Ctrl+`, Ctrl+Tab, Mod+1..9) through instead of sending them to the shell.
    xterm.attachCustomKeyEventHandler((e) => !(e.ctrlKey && (e.key === "`" || e.key === "Tab")) && !((e.metaKey || e.ctrlKey) && /^[1-9]$/.test(e.key)));
    fitAddon.fit();
    term = xterm;
    fit = fitAddon;

    let disposed = false;
    ipc
      .terminalOpen(cwd, xterm.cols, xterm.rows, (e) => {
        if (e.type === "data") {
          xterm.write(decode(e.data));
          noteOutput();
        }
        else {
          exited = true;
          onexit(e.code);
        }
      })
      .then((id) => {
        if (disposed) ipc.terminalClose(id);
        else session = id;
      })
      .catch((e) => toast(errorText(e), "error"));
    const linkProvider = links(xterm);
    const waiting = watchForAgent(xterm);
    const input = xterm.onData((d) => {
      lastInput = Date.now();
      if (session) ipc.terminalWrite(session, d).catch(() => {});
    });
    const resize = xterm.onResize(({ cols, rows }) => session && ipc.terminalResize(session, cols, rows).catch(() => {}));
    const observer = new ResizeObserver(() => host.offsetParent && fitAddon.fit());
    observer.observe(host);

    cleanup = () => {
      disposed = true;
      observer.disconnect();
      input.dispose();
      linkProvider.dispose();
      waiting();
      resize.dispose();
      if (session) ipc.terminalClose(session);
      session = null;
      xterm.dispose();
    };
  });

  onDestroy(() => cleanup?.());

  $effect(() => {
    if (!visible) return;
    requestAnimationFrame(() => {
      fit?.fit();
      term?.focus();
    });
  });

  /** Types `text` at the prompt without pressing Enter. */
  export function type(text: string) {
    if (!session || exited) return;
    ipc.terminalWrite(session, text).catch((e) => toast(errorText(e), "error"));
    term?.focus();
  }

  export function refit() {
    fit?.fit();
  }
</script>

<div class="session" class:hidden={!visible}>
  <div class="host" bind:this={host}></div>
  {#if exited}
    <div class="exited">{t("terminal.exited")}</div>
  {/if}
</div>

<style>
  .session {
    position: absolute;
    inset: 4px 8px 0;
  }
  .session.hidden {
    visibility: hidden;
  }
  .host {
    width: 100%;
    height: 100%;
    -webkit-user-select: text;
    user-select: text;
  }
  .exited {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 8px;
    text-align: center;
    font-size: 11px;
    color: var(--muted);
  }
</style>
