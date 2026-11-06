<script lang="ts">
  // One shell in the project folder. The user drives everything typed into it.
  import { onDestroy, onMount } from "svelte";
  import { Terminal, type ILink } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { errorText, ipc } from "../lib/ipc";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { useProject } from "../lib/project.svelte";
  import { findLinks, projectPath, type TermLink } from "../lib/termLinks";
  import { needsYou } from "../lib/attention.svelte";
  import { agentIn, terminalName } from "../lib/agents";
  import { devServerUrl } from "../lib/scripts";
  import { join } from "../lib/paths";
  import { isMac, toast } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";

  let {
    cwd,
    visible,
    onexit,
    onname,
    onurl,
    run,
    prefill,
    history,
  }: {
    cwd: string;
    visible: boolean;
    onexit: (code: number | null) => void;
    onname: (name: string | null) => void;
    /** A dev server's address showed up in the output. */
    onurl: (url: string) => void;
    /** A command to run once the shell has started (a script from the Scripts menu). */
    run?: string;
    /** Typed at the prompt once the shell has started, without Enter. */
    prefill?: string;
    /** Output from before KlikVibe was last closed, shown dimmed above the new shell. */
    history?: string;
  } = $props();

  // Output is scanned for a dev server's address; the tail of the last chunk joins the next, as an address can be split.
  const scan = new TextDecoder();
  let scanTail = "";
  let lastUrl = "";
  function scanForUrl(bytes: Uint8Array) {
    const text = scanTail + scan.decode(bytes, { stream: true });
    scanTail = text.slice(-120);
    const url = devServerUrl(text);
    if (url && url !== lastUrl) {
      lastUrl = url;
      onurl(url);
    }
  }
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
  let burstNamed = true;

  /** Names the terminal after what runs in front of it (Unix; the shell itself gives no name). */
  async function refreshName() {
    if (!session) return;
    onname(terminalName(await ipc.terminalForeground(session).catch(() => null)));
  }

  function noteOutput() {
    const now = Date.now();
    // The echo of what the user types isn't the agent working.
    if (now - lastInput < 1000) return;
    if (now - lastOutput > QUIET) {
      burstStart = now;
      burstNotified = false;
      burstNamed = false;
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
      if (!session || now - lastOutput < QUIET) return;
      // Whatever ran may have ended (or started) since: the name follows.
      if (!burstNamed) {
        burstNamed = true;
        refreshName();
      }
      if (burstNotified || lastOutput - burstStart < BUSY) return;
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

  /**
   * Web addresses, and files and folders named in the output (only the ones
   * that exist), as links that open with ⌘-click (Ctrl+click): a project file in
   * the editor (at its line), a project folder in the file tree; from outside the
   * project, an image, video or PDF in its default app, anything else in Finder.
   * A line that wraps is read as one, so a long path still links.
   */
  /** Rows above a plain file name searched for the folder it is in. */
  const NEARBY_ROWS = 30;

  function links(xterm: Terminal) {
    const hint = t("term.openHint", { keys: isMac ? "⌘-click" : "Ctrl+click" });
    type Kind = "url" | "file" | "dir" | "outside";
    const open = (l: TermLink, kind: Kind, abs: string) => {
      if (kind === "url") openUrl(l.target).catch((e) => toast(errorText(e), "error"));
      else if (kind === "outside") ipc.openOutside(abs).catch((e) => toast(errorText(e), "error"));
      else if (kind === "dir") project.revealInTree(abs, true);
      else project.openFile(abs).then(() => l.line && project.revealLines(`file:${abs}`, l.line, l.line));
    };
    const absolute = (p: string) => p.startsWith("/") || p.startsWith("~/") || /^[A-Za-z]:[\\/]/.test(p);
    return xterm.registerLinkProvider({
      provideLinks(y, callback) {
        const b = xterm.buffer.active;
        /** The whole logical line around row `row` (0-based): a row that wraps continues the one above. */
        const lineAt = (row: number) => {
          let first = row;
          while (first > 0 && b.getLine(first)?.isWrapped) first--;
          let last = row;
          while (b.getLine(last + 1)?.isWrapped) last++;
          const starts: number[] = [];
          let text = "";
          for (let r = first; r <= last; r++) {
            starts.push(text.length);
            text += b.getLine(r)?.translateToString(r === last) ?? "";
          }
          return { first, last, starts, text };
        };
        const { first, last, starts, text } = lineAt(y - 1);
        /** Buffer position (1-based) of the character at index `i` of `text`. */
        const at = (i: number) => {
          let row = starts.length - 1;
          while (row > 0 && starts[row] > i) row--;
          return { x: i - starts[row] + 1, y: first + row + 1 };
        };
        const link = (l: TermLink, kind: Kind, abs = ""): ILink => ({
          range: { start: at(l.start), end: at(l.end - 1) },
          text: text.slice(l.start, l.end),
          decorations: { underline: true, pointerCursor: true },
          hover: () => (host.title = hint),
          leave: () => (host.title = ""),
          activate: (e) => {
            if (isMac ? e.metaKey : e.ctrlKey) open(l, kind, abs);
          },
        });
        // Only the links on row y (the others come when their rows are asked for).
        const onRow = (l: TermLink) => at(l.start).y <= y && at(l.end - 1).y >= y;
        const ready: ILink[] = [];
        const unsure: { l: TermLink; abs: string }[] = [];
        const outside: TermLink[] = [];
        for (const l of findLinks(text).filter(onRow)) {
          if (l.kind === "url") {
            ready.push(link(l, "url"));
            continue;
          }
          const rel = projectPath(l.target, project.root);
          if (!rel) {
            if (!l.loose && absolute(l.target)) outside.push(l);
            continue;
          }
          if (project.hasFile(rel)) {
            ready.push(link(l, "file", join(project.root, rel)));
            continue;
          }
          // "templates/header.html" printed from somewhere deeper: the one project file ending so.
          const deeper = l.loose ? [] : (project.allFiles?.filter((f) => f.endsWith(`/${rel}`)) ?? []);
          if (deeper.length === 1) ready.push(link(l, "file", join(project.root, deeper[0])));
          // Folders, and what .gitignore leaves out of the file list: asked of the disk.
          else unsure.push({ l, abs: join(project.root, rel) });
        }
        const done = () => callback(ready.length ? ready.sort((a, b) => a.range.start.y - b.range.start.y || a.range.start.x - b.range.start.x) : undefined);
        if (!unsure.length && !outside.length) return done();
        /** Absolute folders printed on this line or the rows above, newest first. */
        const nearbyFolders = () => {
          const found: string[] = [];
          for (let r = last; r >= Math.max(0, first - NEARBY_ROWS) && found.length < 5; ) {
            const line = r === last ? { first, text } : lineAt(r);
            r = line.first - 1;
            for (const l of findLinks(line.text).reverse()) {
              if (l.kind !== "path" || l.loose || !absolute(l.target)) continue;
              // A file's folder counts too: "/x/card/ios-card.png" then "sheet.png".
              const folder = /\.[A-Za-z0-9]+$/.test(l.target) ? l.target.replace(/\/[^/]*$/, "") : l.target;
              if (folder && !found.includes(folder)) found.push(folder);
            }
          }
          return found;
        };
        Promise.all([
          unsure.length
            ? ipc.pathKinds(unsure.map((u) => u.abs)).then((kinds) =>
                unsure.filter((u, i) => {
                  if (kinds[i]) ready.push(link(u.l, kinds[i]!, u.abs));
                  return !kinds[i];
                }),
              )
            : [],
          outside.length
            ? ipc.outsideKinds(outside.map((l) => l.target)).then((kinds) => outside.forEach((l, i) => kinds[i] && ready.push(link(l, "outside", l.target))))
            : null,
        ])
          // A plain file name not in the project ("sheet-ios-feed.png"): in a folder printed just above?
          .then(async ([missing]) => {
            const names = missing.filter((u) => !u.l.loose && !u.l.target.includes("/"));
            const folders = names.length ? nearbyFolders() : [];
            if (!folders.length) return;
            const tries = names.flatMap((u) => folders.map((f) => ({ l: u.l, path: `${f}/${u.l.target}` })));
            const kinds = await ipc.outsideKinds(tries.map((x) => x.path));
            const linked = new Set<TermLink>();
            tries.forEach((x, i) => {
              if (kinds[i] && kinds[i] !== "dir" && !linked.has(x.l)) {
                linked.add(x.l);
                ready.push(link(x.l, "outside", x.path));
              }
            });
          })
          .catch(() => {})
          .finally(done);
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
    if (history) {
      const rule = `── ${t("terminal.restored")} ──`;
      xterm.write(`\x1b[2m${history.replace(/\r?\n/g, "\r\n")}\r\n${rule}\x1b[0m\r\n`);
    }

    let disposed = false;
    ipc
      .terminalOpen(cwd, xterm.cols, xterm.rows, (e) => {
        if (e.type === "data") {
          const bytes = decode(e.data);
          xterm.write(bytes);
          noteOutput();
          scanForUrl(bytes);
        }
        else {
          exited = true;
          onexit(e.code);
        }
      })
      .then((id) => {
        if (disposed) ipc.terminalClose(id);
        else {
          session = id;
          // The user picked this script to run: the shell runs it as if they had typed it.
          if (run) ipc.terminalWrite(id, `${run}\r`).catch(() => {});
          else if (prefill) ipc.terminalWrite(id, prefill).catch(() => {});
        }
      })
      .catch((e) => toast(errorText(e), "error"));
    const linkProvider = links(xterm);
    const waiting = watchForAgent(xterm);
    const input = xterm.onData((d) => {
      lastInput = Date.now();
      // Enter starts a command (or an agent): name the terminal after it once it is running.
      if (d.includes("\r")) setTimeout(refreshName, 700);
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

  /** Pastes `text` (as a paste, so a TUI agent takes it whole and nothing is sent); false when the shell has ended. */
  export function paste(text: string): boolean {
    if (!session || exited || !term) return false;
    term.paste(text);
    term.focus();
    return true;
  }

  /** The last `n` lines of output as plain text (wrapped lines joined back), for the MCP tools. */
  export function tail(n: number): string {
    if (!term) return "";
    const b = term.buffer.active;
    let last = b.length - 1;
    while (last >= 0 && !b.getLine(last)?.translateToString(true).trim()) last--;
    const lines: string[] = [];
    for (let y = last; y >= 0 && lines.length < n; y--) {
      // A wrapped line continues the one above it: walk up to where it starts.
      let text = b.getLine(y)?.translateToString(true) ?? "";
      while (y > 0 && b.getLine(y)?.isWrapped) text = (b.getLine(--y)?.translateToString(true) ?? "") + text;
      lines.push(text);
    }
    return lines.reverse().join("\n");
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
