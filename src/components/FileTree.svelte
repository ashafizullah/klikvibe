<script lang="ts">
  // The project's files: the tree, or search results while there is a query.
  // Search by name runs on the file list in memory (search.ts); search in
  // contents runs in Rust (files.rs) as you type.
  import { onDestroy, onMount, tick } from "svelte";
  import { useProject, workspace } from "../lib/project.svelte";
  import { errorText, ipc, type TextMatch, type TextSearch } from "../lib/ipc";
  import { matchNames } from "../lib/search";
  import { fileIcon } from "../lib/fileIcons";
  import { join } from "../lib/paths";
  import { mod, openMenu, toast } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import TreeNode from "./TreeNode.svelte";
  import Segmented from "./Segmented.svelte";

  const project = useProject();
  const root = project.root;
  const entries = $derived(project.tree.get(root) ?? []);

  // A clock for the "just changed" marks, so they fade without other changes.
  let now = $state(Date.now());
  const timer = setInterval(() => (now = Date.now()), 2000);
  onDestroy(() => clearInterval(timer));

  function refresh() {
    for (const dir of [...project.tree.keys()]) project.loadDir(dir);
    if (project.allFiles) project.loadAllFiles();
  }

  /** Right-click on the empty part of the tree: new things at the project's top. */
  function rootMenu(e: MouseEvent) {
    if ((e.target as Element).closest(".node, .result, .line")) return;
    openMenu(e, [
      { label: t("files.newFile"), icon: "file-plus", run: () => project.newEntry(root, false) },
      { label: t("files.newFolder"), icon: "folder-plus", run: () => project.newEntry(root, true) },
    ]);
  }

  /** Some folder is open: the button closes them all, else it opens them. */
  const anyOpen = $derived(project.expanded.size > 0);
  function toggleAll() {
    if (anyOpen) project.expanded.clear();
    else project.expandAll();
  }

  // ---- search ----

  let query = $state("");
  let include = $state("");
  let mode = $state<"names" | "text">("names");
  let focused = $state(false);
  let input = $state<HTMLInputElement>();
  /** The highlighted result, for the arrow keys and Enter. */
  let cursor = $state(0);
  const searching = $derived(query.trim().length > 0);

  $effect(() => {
    if (searching && mode === "names" && project.allFiles === null) project.loadAllFiles();
  });
  const names = $derived(searching && mode === "names" && project.allFiles ? matchNames(project.allFiles, query) : []);

  let found = $state<TextSearch | null>(null);
  let busy = $state(false);
  let seq = 0;
  $effect(() => {
    const q = query.trim();
    const inc = include.trim();
    if (mode !== "text" || q.length < 2) {
      // A search still on its way is for a query that is gone.
      seq++;
      found = null;
      busy = false;
      return;
    }
    const mine = ++seq;
    busy = true;
    const wait = setTimeout(async () => {
      try {
        const result = await ipc.searchText(root, q, inc || null);
        if (mine === seq) found = result;
      } catch (e) {
        if (mine === seq) {
          found = null;
          toast(errorText(e), "error");
        }
      } finally {
        if (mine === seq) busy = false;
      }
    }, 300);
    return () => clearTimeout(wait);
  });

  const split = (path: string) => {
    const i = path.lastIndexOf("/");
    return { name: path.slice(i + 1), dir: i < 0 ? "" : path.slice(0, i) };
  };
  const groups = $derived.by(() => {
    const out: { path: string; name: string; dir: string; matches: TextMatch[] }[] = [];
    for (const m of found?.matches ?? []) {
      const last = out[out.length - 1];
      if (last?.path === m.path) last.matches.push(m);
      else out.push({ path: m.path, ...split(m.path), matches: [m] });
    }
    return out;
  });
  /** Every result in order, for the keyboard. */
  const flat = $derived(mode === "names" ? names.map((n) => ({ path: n.path, line: 0 })) : (found?.matches ?? []));
  $effect(() => {
    void flat;
    cursor = 0;
  });

  /** Rust reports byte offsets; split the line there (they differ from string offsets past ASCII). */
  function marked(m: TextMatch): [string, string, string] {
    const bytes = new TextEncoder().encode(m.text);
    const dec = new TextDecoder();
    return [dec.decode(bytes.slice(0, m.start)), dec.decode(bytes.slice(m.start, m.end)), dec.decode(bytes.slice(m.end))];
  }

  async function open(path: string, line = 0) {
    const abs = join(root, path);
    await project.openFile(abs);
    if (line) project.revealLines(`file:${abs}`, line);
  }

  let list = $state<HTMLElement>();

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = flat.length;
      if (n) cursor = (cursor + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      // In this project's list: every open project keeps its tree (hidden ones too).
      list?.querySelector(`[data-result="${cursor}"]`)?.scrollIntoView({ block: "nearest" });
    } else if (e.key === "Enter" && flat[cursor]) {
      e.preventDefault();
      open(flat[cursor].path, flat[cursor].line);
    } else if (e.key === "Escape") {
      if (query) query = "";
      else input?.blur();
    }
  }

  // Mod+P (names) and Mod+Shift+F (contents) from App, for the project in front.
  onMount(() => {
    const go = (e: Event) => {
      if (workspace.active !== root) return;
      mode = (e as CustomEvent<"names" | "text">).detail;
      // The panel may have just been shown.
      tick().then(() => {
        input?.focus();
        input?.select();
      });
    };
    window.addEventListener("klikvibe:search", go);
    return () => window.removeEventListener("klikvibe:search", go);
  });
</script>

<section class="tree">
  <div class="panel-head">
    <span class="panel-title">{t("tree.title")}</span>
    <button class="icon-btn" onclick={() => project.newEntry(root, false)} title={t("files.newFileTitle")} aria-label={t("files.newFileTitle")}><Icon name="file-plus" size={13} /></button>
    <button class="icon-btn" onclick={() => project.newEntry(root, true)} title={t("files.newFolderTitle")} aria-label={t("files.newFolderTitle")}><Icon name="folder-plus" size={13} /></button>
    <button class="icon-btn" onclick={refresh} title={t("tree.refresh")} aria-label={t("tree.refresh")}><Icon name="refresh" size={13} /></button>
    <button class="icon-btn" onclick={toggleAll} title={anyOpen ? t("tree.collapse") : t("tree.expand")} aria-label={anyOpen ? t("tree.collapse") : t("tree.expand")}>
      <Icon name={anyOpen ? "collapse-all" : "expand-all"} size={13} />
    </button>
  </div>

  <div class="search">
    <div class="field-row">
      <Icon name="search" size={13} />
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onKey}
        onfocus={() => (focused = true)}
        onblur={() => (focused = false)}
        placeholder={mode === "names" ? t("search.placeholderNames") : t("search.placeholderText")}
        spellcheck="false"
        autocomplete="off"
        aria-label={t("search.title")}
      />
      {#if query}
        <button class="icon-btn clear" onclick={() => ((query = ""), input?.focus())} title={t("search.clear")} aria-label={t("search.clear")}><Icon name="close" size={12} /></button>
      {/if}
    </div>
    {#if searching || focused || mode === "text"}
      <Segmented
        stretch
        keepFocus
        label={t("search.title")}
        value={mode}
        onchange={(m) => (mode = m)}
        options={[
          { value: "names", label: t("search.names"), title: `${t("search.namesHint")} (${mod}P)` },
          { value: "text", label: t("search.text"), title: `${t("search.textHint")} (${mod}⇧F)` },
        ]}
      />
    {/if}
    {#if mode === "text"}
      <input class="include" bind:value={include} onkeydown={onKey} placeholder={t("search.include")} spellcheck="false" autocomplete="off" aria-label={t("search.include")} />
    {/if}
  </div>

  <div class="items" role="tree" tabindex="-1" bind:this={list} oncontextmenu={rootMenu}>
    {#if !searching}
      {#if mode === "names" && focused}<p class="hint">{t("search.hint")}</p>{/if}
      {#each entries as entry (entry.path)}
        <TreeNode {entry} depth={0} {now} />
      {:else}
        <p class="empty-note centered">{t("tree.empty")}</p>
      {/each}
    {:else if mode === "names"}
      {#if project.allFiles === null}
        <p class="empty-note">{t("search.searching")}</p>
      {:else}
        {#each names as n, i (n.path)}
          <button class="result" class:on={i === cursor} data-result={i} onclick={() => open(n.path)} title={n.path}>
            <Icon name={fileIcon(n.name).icon} color={fileIcon(n.name).color} size={13} />
            <span class="name ellipsis">
              {#if n.mark}{n.name.slice(0, n.mark[0])}<mark>{n.name.slice(n.mark[0], n.mark[1])}</mark>{n.name.slice(n.mark[1])}{:else}{n.name}{/if}
            </span>
            <span class="dir ellipsis">{n.dir}</span>
          </button>
        {:else}
          <p class="empty-note">{t("search.none")}</p>
        {/each}
        {#if names.length >= 200 || project.allFilesTruncated}<p class="hint">{t("search.more", { count: names.length })}</p>{/if}
      {/if}
    {:else if query.trim().length < 2}
      <p class="empty-note">{t("search.minChars")}</p>
    {:else if busy && !found}
      <p class="empty-note">{t("search.searching")}</p>
    {:else if found}
      {#if found.matches.length}
        <p class="hint">{t("search.matches", { count: found.matches.length, files: groups.length })}{busy ? " …" : ""}</p>
      {/if}
      {#each groups as g (g.path)}
        <div class="group" title={g.path}>
          <Icon name={fileIcon(g.name).icon} color={fileIcon(g.name).color} size={13} />
          <span class="name ellipsis">{g.name}</span>
          <span class="dir ellipsis">{g.dir}</span>
          <span class="count-badge">{g.matches.length}</span>
        </div>
        {#each g.matches as m (m.line)}
          {@const i = found.matches.indexOf(m)}
          {@const [before, hit, after] = marked(m)}
          <button class="line" class:on={i === cursor} data-result={i} onclick={() => open(m.path, m.line)}>
            <span class="no">{m.line}</span>
            <span class="text">{before}<mark>{hit}</mark>{after}</span>
          </button>
        {/each}
      {:else}
        <p class="empty-note">{t("search.none")}</p>
      {/each}
      {#if found.truncated}<p class="hint">{t("search.more", { count: found.matches.length })}</p>{/if}
    {/if}
  </div>
</section>

<style>
  .tree {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    min-width: 0;
  }
  .search {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 8px 6px;
    border-bottom: 1px solid var(--line);
  }
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 4px 0 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--bg);
    color: var(--muted);
  }
  .field-row:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
    font: 12px var(--sans);
    -webkit-user-select: text;
    user-select: text;
  }
  input::placeholder {
    color: var(--muted);
  }
  .clear {
    width: 20px;
    height: 20px;
  }
  .include {
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--bg);
    font: 11.5px var(--mono);
  }
  .include:focus {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .items {
    flex: 1;
    overflow: auto;
    padding: 4px 0 12px;
    display: flex;
    flex-direction: column;
  }
  .items > :global(*) {
    flex: none;
  }
  /* An empty project: the note sits in the middle of the panel. */
  .items > .centered {
    margin: auto;
    text-align: center;
  }
  .hint {
    margin: 2px 12px 6px;
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
  }
  .result,
  .line,
  .group {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    border: none;
    background: none;
    text-align: left;
  }
  .result {
    height: 24px;
    padding: 0 12px;
    font-size: 12.5px;
  }
  .result :global(svg),
  .group :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .name {
    flex: none;
    max-width: 65%;
  }
  .dir {
    flex: 1;
    font-size: 10.5px;
    color: var(--muted);
  }
  .group {
    height: 26px;
    margin-top: 4px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 500;
  }
  .line {
    align-items: baseline;
    padding: 2px 12px 2px 20px;
    font: 11.5px/1.5 var(--mono);
  }
  .line .no {
    flex: none;
    min-width: 28px;
    text-align: right;
    color: var(--muted);
  }
  .line .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
    color: var(--text);
  }
  .result:hover,
  .line:hover {
    background: var(--panel-2);
  }
  .result.on,
  .line.on {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  mark {
    color: inherit;
    border-radius: 2px;
    background: color-mix(in srgb, var(--accent) 38%, transparent);
  }
</style>
