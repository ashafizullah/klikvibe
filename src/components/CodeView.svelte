<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { basicSetup } from "codemirror";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { indentWithTab } from "@codemirror/commands";
  import { darkroomHighlight, darkroomTheme } from "../lib/editor";
  import { loadLanguage } from "../lib/languages";
  import { isDirty, useProject, type Tab } from "../lib/project.svelte";
  import { settings } from "../lib/settings.svelte";
  import { selectAll } from "@codemirror/commands";
  import { gotoLine, openSearchPanel } from "@codemirror/search";
  import { errorText, ipc, type TextMatch } from "../lib/ipc";
  import { componentFiles, importedFrom, moduleAt, rankDefinitions, resolveImport, rustModule } from "../lib/definition";
  import { fileUrl, forgeName } from "../lib/forge";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { join, relative } from "../lib/paths";
  import { copyText, isMac, mod, modAlt, openMenu, reveal, revealLabel, toast, ui, type MenuItem } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";

  let { tab }: { tab: Tab } = $props();
  const project = useProject();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  const language = new Compartment();
  const wrap = new Compartment();
  const size = new Compartment();
  const readOnly = new Compartment();
  /** The tab revision the editor last showed, to tell reloads from typing. */
  let shown = -1;

  const fontTheme = (px: number) => EditorView.theme({ "&": { fontSize: `${px}px` } });

  onMount(() => {
    shown = tab.revision;
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: tab.text,
        extensions: [
          basicSetup,
          keymap.of([
            indentWithTab,
            { key: "Mod-s", preventDefault: true, run: () => (project.saveTab(tab), true) },
            { key: "F12", preventDefault: true, run: (v) => (goToDefinition(v, v.state.selection.main.head), true) },
          ]),
          // ⌘-click (Ctrl+click) goes to a definition, as in other editors; Alt+click adds a cursor.
          EditorView.clickAddsSelectionRange.of((e) => e.altKey),
          EditorView.domEventHandlers({
            mousedown(e, v) {
              if (e.button !== 0 || !(isMac ? e.metaKey : e.ctrlKey)) return false;
              const pos = v.posAtCoords({ x: e.clientX, y: e.clientY });
              if (pos === null) return false;
              e.preventDefault();
              goToDefinition(v, pos, { x: e.clientX, y: e.clientY });
              return true;
            },
          }),
          darkroomTheme,
          darkroomHighlight,
          language.of([]),
          wrap.of(settings.wrap ? EditorView.lineWrapping : []),
          size.of(fontTheme(settings.fontSize)),
          readOnly.of(EditorState.readOnly.of(tab.kind !== "file")),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) tab.text = u.state.doc.toString();
          }),
        ],
      }),
    });
    project.attachView(tab.id, view);
    loadLanguage(tab.title).then((lang) => {
      if (lang && view) view.dispatch({ effects: language.reconfigure(lang) });
    });
  });

  // Holding ⌘ (Ctrl) shows a hand over the code: a click goes to a definition.
  let linking = $state(false);
  const modKey = (e: KeyboardEvent) => (linking = isMac ? e.metaKey : e.ctrlKey);
  const noMod = () => (linking = false);
  window.addEventListener("keydown", modKey);
  window.addEventListener("keyup", modKey);
  window.addEventListener("blur", noMod);

  /**
   * Go to definition, without a language server: in an import's quotes, the file it
   * names; on a word, where it is defined (the file it was imported from first, then
   * this file, then the project). Several places: a menu to pick from.
   */
  async function goToDefinition(v: EditorView, pos: number, at?: { x: number; y: number }) {
    const rel = relative(project.root, tab.path);
    if (rel === null) return;
    if (!project.allFiles) await project.loadAllFiles();
    const files = new Set(project.allFiles ?? []);
    const open = (path: string, line?: number) => {
      const abs = join(project.root, path);
      project.openFile(abs).then(() => line && project.revealLines(`file:${abs}`, line, line));
    };
    const line = v.state.doc.lineAt(pos);
    const spec = moduleAt(line.text, pos - line.from);
    if (spec) {
      const target = resolveImport(spec, rel, files);
      if (target) open(target);
      else toast(t("def.noModule", { name: spec }));
      return;
    }
    const word = v.state.wordAt(pos);
    if (!word) return;
    const name = v.state.sliceDoc(word.from, word.to);
    const rustMod = rel.endsWith(".rs") ? rustModule(line.text, name, rel, files) : null;
    if (rustMod) return open(rustMod);
    const from = importedFrom(v.state.doc.toString(), name);
    const imported = from ? resolveImport(from, rel, files) : null;
    // A component imported from its own file (import Button from "./Button.svelte"): that file.
    if (imported && /\.(svelte|vue)$/.test(imported)) return open(imported);
    let found: TextMatch[];
    try {
      found = await ipc.findDefinitions(project.root, name);
    } catch (e) {
      toast(errorText(e), "error");
      return;
    }
    if (!found.length) found = componentFiles(name, files).map((path) => ({ path, line: 1, text: path, start: 0, end: 0 }));
    if (!found.length) return toast(t("def.none", { name }));
    const { list, sure } = rankDefinitions(found, rel, imported);
    if (sure) return open(list[0].path, list[0].line);
    const where = at ?? v.coordsAtPos(pos) ?? { left: 0, bottom: 0 };
    const x = "x" in where ? where.x : where.left;
    const y = "y" in where ? where.y : where.bottom;
    ui.menu = {
      x,
      y,
      items: list.slice(0, 20).map((m) => ({ label: `${m.path}:${m.line}`, hint: m.text.trim().slice(0, 48), run: () => open(m.path, m.line) })),
    };
  }

  onDestroy(() => {
    window.removeEventListener("keydown", modKey);
    window.removeEventListener("keyup", modKey);
    window.removeEventListener("blur", noMod);
    if (view) project.detachView(tab.id, view);
    view?.destroy();
  });

  // A reload from disk (an agent's edit): swap the text in, keeping the cursor near where it was.
  $effect(() => {
    const rev = tab.revision;
    if (!view || rev === shown) return;
    shown = rev;
    const current = view.state.doc.toString();
    if (current === tab.text) return;
    const head = Math.min(view.state.selection.main.head, tab.text.length);
    const top = view.scrollDOM.scrollTop;
    view.dispatch({ changes: { from: 0, to: current.length, insert: tab.text }, selection: { anchor: head } });
    view.scrollDOM.scrollTop = top;
  });

  $effect(() => {
    view?.dispatch({ effects: wrap.reconfigure(settings.wrap ? EditorView.lineWrapping : []) });
  });
  $effect(() => {
    view?.dispatch({ effects: size.reconfigure(fontTheme(settings.fontSize)) });
  });

  /** The editor's right-click menu: editing, then the file. */
  function menu(e: MouseEvent) {
    const v = view;
    if (!v) return;
    const sel = v.state.selection.main;
    const selected = v.state.sliceDoc(sel.from, sel.to);
    const editable = !v.state.readOnly;
    const change = project.changeFor(tab.path);
    const done = () => v.focus();
    const rel = relative(project.root, tab.path) ?? tab.path;
    const first = v.state.doc.lineAt(sel.from).number;
    // A selection ending at the start of a line doesn't include that line.
    const last = v.state.doc.lineAt(sel.empty ? sel.to : Math.max(sel.from, sel.to - 1)).number;
    const about = sel.empty
      ? `@${rel} `
      : first === last
        ? `${t("ask.line", { path: `@${rel}`, line: first })} `
        : `${t("ask.lines", { path: `@${rel}`, from: first, to: last })} `;
    const items: MenuItem[] = [
      { label: t("ask.agent"), icon: "sparkles", color: "var(--accent)", run: () => project.askAgent?.(about) },
      { label: t("def.go"), hint: "F12", run: () => goToDefinition(v, sel.head, { x: e.clientX, y: e.clientY }) },
      "sep",
      {
        label: t("edit.cut"),
        hint: `${mod}X`,
        disabled: !editable || sel.empty,
        run: () => copyText(selected, true).then(() => (v.dispatch(v.state.replaceSelection("")), done())),
      },
      { label: t("edit.copy"), hint: `${mod}C`, disabled: sel.empty, run: () => copyText(selected, true).then(done) },
      {
        label: t("edit.paste"),
        hint: `${mod}V`,
        disabled: !editable,
        run: () =>
          ipc
            .clipboardText()
            .then((text) => text && v.dispatch(v.state.replaceSelection(text)))
            .catch((err) => toast(errorText(err), "error"))
            .finally(done),
      },
      "sep",
      { label: t("edit.selectAll"), hint: `${mod}A`, run: () => (selectAll(v), done()) },
      { label: t("edit.find"), hint: `${mod}F`, run: () => openSearchPanel(v) },
      { label: t("edit.gotoLine"), hint: modAlt("G"), run: () => gotoLine(v) },
      "sep",
    ];
    if (isDirty(tab)) items.push({ label: t("tabs.save"), hint: `${mod}S`, run: () => project.saveTab(tab) });
    const web = project.remote && project.git?.branch ? fileUrl(project.remote, project.git.branch, rel, first, sel.empty ? undefined : last) : null;
    if (web && project.remote) {
      const pushed = !!project.git?.upstream;
      items.push({ label: t("forge.open", { name: forgeName(project.remote) }), disabled: !pushed, hint: pushed ? undefined : "↑", run: () => openUrl(web) });
    }
    if (change) items.push({ label: t("git.menu.openDiff"), run: () => project.openDiff(change) });
    items.push(
      { label: t("menu.copyPath"), run: () => copyText(tab.path) },
      { label: t("menu.copyRelative"), run: () => copyText(rel) },
      { label: revealLabel(), run: () => reveal(tab.path) },
    );
    openMenu(e, items);
  }

  export function focus() {
    view?.focus();
  }
</script>

<div class="code" class:linking bind:this={host} oncontextmenu={menu} role="presentation"></div>

<style>
  .code {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    -webkit-user-select: text;
    user-select: text;
  }
  .code :global(.cm-editor) {
    height: 100%;
  }
  .code.linking :global(.cm-content) {
    cursor: pointer;
  }
  .code :global(.cm-editor.cm-focused) {
    outline: none;
  }
</style>
