<script lang="ts">
  import type { Entry } from "../lib/ipc";
  import { useProject } from "../lib/project.svelte";
  import { mediaKind } from "../lib/media";
  import { fileIcon } from "../lib/fileIcons";
  import { errorText, ipc } from "../lib/ipc";
  import { dirName, relative } from "../lib/paths";
  import { copyText, openMenu, reveal, revealLabel, toast, type MenuItem } from "../lib/ui.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import TreeNode from "./TreeNode.svelte";

  let { entry, depth, now }: { entry: Entry; depth: number; now: number } = $props();
  const project = useProject();

  const open = $derived(entry.dir && project.expanded.has(entry.path));
  const children = $derived(open ? (project.tree.get(entry.path) ?? []) : []);
  const change = $derived(entry.dir ? undefined : project.changeFor(entry.path));
  const changedInside = $derived(entry.dir && project.folderChanged(entry.path));
  const fresh = $derived(project.isFresh(entry.path, now));
  const active = $derived(project.active === `file:${entry.path}` || project.active === `diff:${entry.path}`);
  function menu(e: MouseEvent) {
    const rel = relative(project.root, entry.path) ?? entry.path;
    // New things go in this folder, or next to this file.
    const folder = entry.dir ? entry.path : dirName(entry.path);
    const manage: MenuItem[] = [
      "sep",
      { label: t("files.newFile"), icon: "file-plus", run: () => project.newEntry(folder, false) },
      { label: t("files.newFolder"), icon: "folder-plus", run: () => project.newEntry(folder, true) },
      "sep",
      { label: t("files.renameMenu"), run: () => project.renameEntry(entry.path, entry.dir) },
      { label: t("files.deleteMenu"), run: () => project.deleteEntry(entry.path, entry.dir) },
    ];
    const where: MenuItem[] = [
      { label: t("menu.copyPath"), run: () => copyText(entry.path) },
      { label: t("menu.copyRelative"), run: () => copyText(rel) },
      { label: revealLabel(), run: () => reveal(entry.path) },
    ];
    if (entry.dir) {
      openMenu(e, [
        { label: t("ask.agent"), icon: "sparkles", color: "var(--accent)", run: () => project.askAgent?.(`@${rel}/ `) },
        "sep",
        { label: open ? t("menu.collapse") : t("menu.expand"), run: () => project.toggleDir(entry.path) },
        ...manage,
        "sep",
        ...where,
      ]);
      return;
    }
    const c = change;
    const git: MenuItem[] = c
      ? [
          "sep",
          { label: c.kind === "untracked" ? t("git.menu.delete") : t("git.menu.discard"), disabled: c.kind === "conflict", run: () => project.discard(c) },
          ...(c.kind === "untracked" ? [{ label: t("git.menu.ignore"), run: () => project.ignore(c.path) }] : []),
        ]
      : [];
    openMenu(e, [
      { label: t("ask.agent"), icon: "sparkles", color: "var(--accent)", run: () => project.askAgent?.(`@${rel} `) },
      "sep",
      { label: t("menu.open"), run: () => project.openFile(entry.path) },
      ...(c ? [{ label: t("git.menu.openDiff"), run: () => project.openDiff(c) }] : []),
      ...(mediaKind(entry.name)
        ? [{ label: t("media.openExternally"), run: () => ipc.openExternally(entry.path).catch((err) => toast(errorText(err), "error")) }]
        : []),
      ...git,
      ...manage,
      "sep",
      ...where,
    ]);
  }

  const MARK: Record<string, string> = { added: "A", modified: "M", deleted: "D", renamed: "R", untracked: "U", conflict: "!" };
</script>

<button
  class="node"
  data-path={entry.path}
  class:active
  class:fresh
  style:padding-left="{8 + depth * 14}px"
  onclick={() => (entry.dir ? project.toggleDir(entry.path) : project.openFile(entry.path))}
  oncontextmenu={menu}
  title={entry.path}
>
  {#if entry.dir}
    <Icon name={open ? "down" : "right"} size={12} class="chev" />
    <Icon name={open ? "open" : "folder"} size={14} class="kind dir" />
  {:else}
    <span class="chev-space"></span>
    {@const fi = fileIcon(entry.name)}
    <Icon name={fi.icon} color={fi.color} size={14} class="kind" />
  {/if}
  <span class="label-text ellipsis {change?.kind ?? ''}" class:inside={changedInside}>{entry.name}</span>
  {#if fresh}<span class="fresh-dot" aria-hidden="true"></span>{/if}
  {#if change}<span class="git-mark {change.kind}">{MARK[change.kind]}</span>{/if}
</button>
{#if open}
  {#each children as child (child.path)}
    <TreeNode entry={child} depth={depth + 1} {now} />
  {/each}
{/if}

<style>
  .node {
    width: 100%;
    height: 24px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding-right: 8px;
    border: none;
    background: none;
    text-align: left;
    font-size: 12.5px;
    color: var(--text);
    white-space: nowrap;
  }
  .node:hover {
    background: color-mix(in srgb, var(--panel-2) 80%, transparent);
  }
  .node.active {
    background: var(--selection);
  }
  .node :global(.chev) {
    color: var(--muted);
    flex: none;
  }
  .chev-space {
    width: 12px;
    flex: none;
  }
  .node :global(.kind) {
    color: var(--muted);
    flex: none;
  }
  .node :global(.kind.dir) {
    color: color-mix(in srgb, var(--accent) 75%, var(--muted));
  }
  .label-text {
    flex: 1;
  }
  .label-text.modified,
  .label-text.renamed,
  .label-text.inside {
    color: var(--accent);
  }
  .label-text.added,
  .label-text.untracked {
    color: var(--ok);
  }
  .label-text.deleted,
  .label-text.conflict {
    color: var(--danger);
  }
  /* Changed on disk a moment ago: the agent is working here. */
  .fresh-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
    flex: none;
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
