<script lang="ts">
  // One shell in the project folder. The user drives everything typed into it.
  import { onDestroy, onMount } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { errorText, ipc } from "../lib/ipc";
  import { toast } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";

  let { cwd, visible, onexit }: { cwd: string; visible: boolean; onexit: (code: number | null) => void } = $props();

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
    // Let the app-level Ctrl+` toggle through instead of sending it to the shell.
    xterm.attachCustomKeyEventHandler((e) => !(e.ctrlKey && e.key === "`"));
    fitAddon.fit();
    term = xterm;
    fit = fitAddon;

    let disposed = false;
    ipc
      .terminalOpen(cwd, xterm.cols, xterm.rows, (e) => {
        if (e.type === "data") xterm.write(decode(e.data));
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
    const input = xterm.onData((d) => session && ipc.terminalWrite(session, d).catch(() => {}));
    const resize = xterm.onResize(({ cols, rows }) => session && ipc.terminalResize(session, cols, rows).catch(() => {}));
    const observer = new ResizeObserver(() => host.offsetParent && fitAddon.fit());
    observer.observe(host);

    cleanup = () => {
      disposed = true;
      observer.disconnect();
      input.dispose();
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
  <div class="host grain-free" bind:this={host}></div>
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
