<script lang="ts">
  import { onDestroy } from "svelte";
  import { useProject } from "../lib/project.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
  import TreeNode from "./TreeNode.svelte";

  const project = useProject();
  const root = project.root;
  const entries = $derived(project.tree.get(root) ?? []);

  // A clock for the "just changed" marks, so they fade without other changes.
  let now = $state(Date.now());
  const timer = setInterval(() => (now = Date.now()), 2000);
  onDestroy(() => clearInterval(timer));

  function refresh() {
    for (const dir of [...project.tree.keys()]) project.loadDir(dir);
  }

  function collapse() {
    project.expanded.clear();
  }
</script>

<section class="tree">
  <div class="panel-head">
    <span class="panel-title">{t("tree.title")}</span>
    <button class="icon-btn" onclick={refresh} title={t("tree.refresh")} aria-label={t("tree.refresh")}><Icon name="refresh" size={13} /></button>
    <button class="icon-btn" onclick={collapse} title={t("tree.collapse")} aria-label={t("tree.collapse")}><Icon name="down" size={13} /></button>
  </div>
  <div class="items" role="tree">
    {#each entries as entry (entry.path)}
      <TreeNode {entry} depth={0} {now} />
    {:else}
      <p class="empty-note">{t("tree.empty")}</p>
    {/each}
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
  .items {
    flex: 1;
    overflow: auto;
    padding: 4px 0 12px;
  }
</style>
