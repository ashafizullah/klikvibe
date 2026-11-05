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
  import { errorText, ipc } from "../lib/ipc";
  import { relative } from "../lib/paths";
  import { copyText, mod, modAlt, openMenu, reveal, revealLabel, toast, type MenuItem } from "../lib/ui.svelte";
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
          keymap.of([indentWithTab, { key: "Mod-s", preventDefault: true, run: () => (project.saveTab(tab), true) }]),
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

  onDestroy(() => {
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
    const items: MenuItem[] = [
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
    if (change) items.push({ label: t("git.menu.openDiff"), run: () => project.openDiff(change) });
    items.push(
      { label: t("menu.copyPath"), run: () => copyText(tab.path) },
      { label: t("menu.copyRelative"), run: () => copyText(relative(project.root, tab.path) ?? tab.path) },
      { label: revealLabel(), run: () => reveal(tab.path) },
    );
    openMenu(e, items);
  }

  export function focus() {
    view?.focus();
  }
</script>

<div class="code" bind:this={host} oncontextmenu={menu} role="presentation"></div>

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
  .code :global(.cm-editor.cm-focused) {
    outline: none;
  }
</style>
