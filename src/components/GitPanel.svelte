<script lang="ts">
  import { useProject } from "../lib/project.svelte";
  import { ago } from "../lib/time";
  import { i18n, t, type Key } from "../lib/i18n/index.svelte";
  import { copyText, mod, openMenu, openMenuBelow, reveal, revealLabel, type MenuItem } from "../lib/ui.svelte";
  import { branchUrl, commitUrl, forgeName, isMergeRequest, newRequestUrl } from "../lib/forge";
  import { mergeTask, newTask, openTask, removeTask } from "../lib/project.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { ipc } from "../lib/ipc";
  import type { Change, Commit, Worktree } from "../lib/ipc";
  import { join } from "../lib/paths";
  import Icon from "./Icon.svelte";
  import { fileIcon } from "../lib/fileIcons";
  import { isLastPanel, togglePanel } from "../lib/layout";
  import { layoutGraph, refLabels } from "../lib/graph";
  import { persist, settings } from "../lib/settings.svelte";
  import GraphCell, { graphWidth, laneColor, laneX } from "./GraphCell.svelte";
  import Segmented from "./Segmented.svelte";

  const project = useProject();

  const git = $derived(project.git);
  const MARK: Record<string, string> = { added: "A", modified: "M", deleted: "D", renamed: "R", untracked: "U", conflict: "!" };

  /** Checkpoints newer than this were taken since the user last opened the tab. */
  let newSince = $state(project.checkpointsSeen);
  // A partly staged file shows in both lists.
  const staged = $derived(git ? git.changes.filter((c) => c.staged) : []);
  const working = $derived(git ? git.changes.filter((c) => c.unstaged) : []);
  const unseen = $derived(git ? git.changes.filter((c) => project.isUnseen(c.path)).length : 0);

  function setTab(tab: "changes" | "history" | "checkpoints") {
    if (tab === "checkpoints" && project.gitTab !== "checkpoints") {
      newSince = project.checkpointsSeen;
      project.checkpointsSeen = Date.now();
    }
    project.gitTab = tab;
    if (tab === "history") project.refreshGit(true);
    if (tab === "checkpoints") project.loadCheckpoints();
  }

  const REASON_ICON: Record<string, string> = { opened: "open", changes: "diff", "before-restore": "save", restored: "rotate-ccw", manual: "save", agent: "sparkles" };

  /** "14:05", or "3 Nov 14:05" when it isn't today. */
  function moment(iso: string) {
    const d = new Date(iso);
    const time = d.toLocaleTimeString(i18n.locale, { hour: "2-digit", minute: "2-digit" });
    return d.toDateString() === new Date().toDateString() ? time : `${d.toLocaleDateString(i18n.locale, { day: "numeric", month: "short" })} ${time}`;
  }

  const KEEP = [50, 100, 200, 500, 0];
  const megabytes = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))} KB` : `${(b / 1024 / 1024).toFixed(1)} MB`);
  const usage = $derived.by(() => {
    const count = project.checkpoints?.length ?? 0;
    const head = settings.checkpointLimit ? t("checkpoints.usage", { count, limit: settings.checkpointLimit }) : String(count);
    return project.checkpointBytes === null ? head : `${head} · ${megabytes(project.checkpointBytes)}`;
  });

  function limitMenu(e: MouseEvent) {
    openMenuBelow(e.currentTarget as Element, [
      { label: t("checkpoints.keepTitle"), disabled: true, run: () => {} },
      ...KEEP.map((n) => ({
        label: n ? t("checkpoints.keep", { n }) : t("checkpoints.keepAll"),
        icon: settings.checkpointLimit === n ? "check" : undefined,
        run: () => project.setCheckpointLimit(n),
      })),
    ]);
  }

  function setCheckpoints(on: boolean) {
    settings.checkpoints = on;
    persist();
    if (on) project.takeCheckpoint("manual");
  }

  function onKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      project.commit();
    }
  }

  function changeMenu(e: MouseEvent, c: Change) {
    const abs = join(project.root, c.path);
    const gone = c.kind === "deleted";
    const untracked = c.kind === "untracked";
    const dir = split(c.path).dir;
    openMenu(e, [
      { label: t("ask.changes"), icon: "sparkles", color: "var(--accent)", run: () => project.askAgent?.(`${t("ask.uncommitted", { path: `@${c.path}` })} `) },
      "sep",
      { label: t("git.menu.openDiff"), run: () => project.openDiff(c) },
      ...(c.unstaged && c.kind !== "conflict" ? [{ label: t("git.stage"), run: () => project.stage([c.path]) }] : []),
      ...(c.staged ? [{ label: t("git.unstage"), run: () => project.unstage([c.path]) }] : []),
      { label: t("git.menu.openFile"), disabled: gone, run: () => project.openFile(abs) },
      "sep",
      { label: untracked ? t("git.menu.delete") : t("git.menu.discard"), disabled: c.kind === "conflict", run: () => project.discard(c) },
      ...(untracked
        ? [
            { label: t("git.menu.ignore"), run: () => project.ignore(c.path) },
            ...(dir ? [{ label: t("git.menu.ignoreFolder", { dir }), run: () => project.ignore(dir, true) }] : []),
          ]
        : []),
      "sep",
      { label: t("menu.copyPath"), run: () => copyText(abs) },
      { label: t("menu.copyRelative"), run: () => copyText(c.path) },
      { label: revealLabel(), disabled: gone, run: () => reveal(abs) },
    ]);
  }

  const graph = $derived(layoutGraph(project.log));
  /** One width for the whole column, so the text lines up; very wide graphs are cut at 8 lanes. */
  const graphW = $derived(graphWidth(Math.min(8, Math.max(1, ...graph.map((r) => r.width)))));

  /** The lanes that run on past row `i` (into the next row), for the strip beside its file list. */
  function lanesAfter(i: number) {
    const next = graph[i + 1]?.top.map((s) => ({ lane: s.from, color: s.color }));
    const own = graph[i]?.bottom.map((s) => ({ lane: s.to, color: s.color })) ?? [];
    const all = next ?? own;
    return all.filter((l, k) => all.findIndex((m) => m.lane === l.lane) === k);
  }

  function setScope(all: boolean) {
    settings.historyAll = all;
    persist();
    project.refreshGit(true);
  }

  /** Parallel tasks: a new one, the others to open, and for a task, merging it back or removing it. */
  async function tasksMenu(el: Element) {
    const list: Worktree[] = await ipc.worktreeList(project.root).catch(() => []);
    const main = list.find((w) => w.main);
    const here = list.find((w) => w.path === project.root);
    const items: MenuItem[] = [{ label: t("tasks.new"), icon: "plus", run: () => newTask(project) }];
    const others = list.filter((w) => w.path !== project.root);
    if (others.length) {
      items.push("sep");
      for (const w of others) {
        const folder = w.path.split(/[\\/]/).pop() ?? w.path;
        items.push({ label: w.main ? t("tasks.mainFolder", { name: folder }) : (w.branch ?? folder), icon: w.main ? "project" : "branch", hint: w.main ? undefined : folder, run: () => openTask(project, w.path) });
      }
    }
    if (here && !here.main && here.branch) {
      const branch = here.branch;
      const into = main?.branch ?? "main";
      items.push("sep", { label: t("tasks.mergeInto", { into }), icon: "pull", run: () => mergeTask(project, branch, into) }, { label: t("tasks.removeThis"), run: () => removeTask(project) });
    }
    openMenuBelow(el, items);
  }

  /** The repository, this branch and a new pull (merge) request on the web. */
  function webMenu(el: Element) {
    const r = project.remote;
    if (!r) return;
    const name = forgeName(r);
    const branch = git?.branch;
    const pushed = !!git?.upstream;
    const br = branch ? branchUrl(r, branch) : null;
    const pr = branch ? newRequestUrl(r, branch) : null;
    const items: MenuItem[] = [{ label: t("forge.openRepo", { name }), run: () => openUrl(r.base) }];
    if (br) items.push({ label: t("forge.openBranch", { name }), disabled: !pushed, hint: pushed ? undefined : "↑", run: () => openUrl(br) });
    if (pr) items.push("sep", { label: t(isMergeRequest(r) ? "forge.newMR" : "forge.newPR", { name }), disabled: !pushed, run: () => openUrl(pr) });
    openMenuBelow(el, items);
  }

  function commitMenu(e: MouseEvent, c: Commit) {
    openMenu(e, [
      { label: t("ask.changes"), icon: "sparkles", color: "var(--accent)", run: () => project.askAgent?.(`${t("ask.commit", { hash: c.short })} `) },
      "sep",
      { label: t("git.menu.openCommit"), run: () => project.openCommit(c) },
      ...(project.remote && commitUrl(project.remote, c.hash)
        ? [{ label: t("forge.open", { name: forgeName(project.remote) }), disabled: c.local, hint: c.local ? "↑" : undefined, run: () => openUrl(commitUrl(project.remote!, c.hash)!) }]
        : []),
      "sep",
      { label: t("menu.copyHash"), run: () => copyText(c.hash) },
      { label: t("git.menu.copyMessage"), run: () => copyText(c.subject) },
    ]);
  }

  const split = (path: string) => {
    const i = path.lastIndexOf("/");
    return i < 0 ? { name: path, dir: "" } : { name: path.slice(i + 1), dir: path.slice(0, i) };
  };
</script>

<!-- The file's type, as in the file tree. -->
{#snippet typeIcon(name: string)}
  {@const fi = fileIcon(name)}
  <Icon name={fi.icon} color={fi.color} size={13} class="type-icon" />
{/snippet}

{#snippet changeRow(c: Change, inIndex: boolean)}
  {@const p = split(c.path)}
  <div class="change">
    <button class="change-main" onclick={() => project.openDiff(c)} oncontextmenu={(e) => changeMenu(e, c)} title={c.from ? `${c.from} → ${c.path}` : c.path}>
      <span class="unseen" class:on={project.isUnseen(c.path)} title={project.isUnseen(c.path) ? t("unseen.dot") : undefined}></span>
      {@render typeIcon(p.name)}
      <span class="name ellipsis">{p.name}</span>
      <span class="dir ellipsis">{p.dir}</span>
    </button>
    <span class="row-actions">
      {#if inIndex}
        <button class="icon-btn" onclick={() => project.unstage([c.path])} title={t("git.unstage")} aria-label={t("git.unstage")}><Icon name="minus" size={13} /></button>
      {:else}
        <button
          class="icon-btn"
          onclick={() => project.discard(c)}
          disabled={c.kind === "conflict"}
          title={c.kind === "untracked" ? t("git.menu.delete") : t("git.menu.discard")}
          aria-label={c.kind === "untracked" ? t("git.menu.delete") : t("git.menu.discard")}
        >
          <Icon name="rotate-ccw" size={12} />
        </button>
        <button class="icon-btn" onclick={() => project.stage([c.path])} title={t("git.stage")} aria-label={t("git.stage")}><Icon name="plus" size={13} /></button>
      {/if}
    </span>
    <span class="git-mark {c.kind}" title={t(`git.kind.${c.kind}` as Key)}>{MARK[c.kind]}</span>
  </div>
{/snippet}

{#snippet fileList(c: { hash: string; short: string }, full = true)}
  {@const files = project.commitFiles.get(c.hash)}
  {#if files === "loading" || files === undefined}
    <p class="files-note">{t("common.loading")}</p>
  {:else if files === "error"}
    <p class="files-note">{t("git.filesError")}</p>
  {:else}
    {#each files as f (f.path)}
      {@const p = split(f.path)}
      <button class="file" onclick={() => project.openCommitFile(c, f)} title={f.from ? `${f.from} → ${f.path}` : f.path}>
        <span class="git-mark {f.kind}" title={t(`git.kind.${f.kind}` as Key)}>{MARK[f.kind]}</span>
        {@render typeIcon(p.name)}
        <span class="name ellipsis">{p.name}</span>
        <span class="dir ellipsis">{p.dir}</span>
        {#if f.added !== null}
          <span class="lines add">+{f.added}</span><span class="lines del">−{f.removed}</span>
        {:else}
          <span class="lines">{t("git.binary")}</span>
        {/if}
      </button>
    {:else}
      <p class="files-note">{t("git.noFileChanges")}</p>
    {/each}
  {/if}
  {#if full}<button class="full" onclick={() => project.openCommit(c)}><Icon name="commit" size={12} />{t("git.showFullCommit")}</button>{/if}
{/snippet}

<section class="git">
  <div class="panel-head">
    <span class="panel-title">Git</span>
    <button class="icon-btn" onclick={() => project.refreshGit(true)} title={t("git.refresh")} aria-label={t("git.refresh")}><Icon name="refresh" size={13} /></button>
    <button class="icon-btn" onclick={() => togglePanel("showGit")} disabled={isLastPanel("showGit")} title={`${t("git.hide")} (${mod}⇧G)`} aria-label={t("git.hide")}>
      <Icon name="close" size={13} />
    </button>
  </div>

  {#if !git}
    <p class="empty-note">{t("common.loading")}</p>
  {:else if !git.repo}
    <div class="norepo">
      <Icon name="branch" size={26} stroke={1.5} />
      <p>{t("git.notRepo")}</p>
      <button class="btn primary" onclick={project.initRepo} disabled={!!project.gitBusy}><Icon name="plus" size={13} />{t("git.init")}</button>
    </div>
  {:else}
    <div class="sync">
      <div class="branch">
        <Icon name="branch" size={13} />
        <span class="mono">{git.branch ?? t("git.detached")}</span>
        {#if git.upstream}<span class="upstream ellipsis mono">→ {git.upstream}</span>{/if}
        <button class="icon-btn tasks" class:alone={!project.remote} onclick={(e) => tasksMenu(e.currentTarget)} title={t("tasks.title")} aria-label={t("tasks.title")}>
          <Icon name="branch-plus" size={12} />
        </button>
        {#if project.remote}
          <button class="icon-btn web" onclick={(e) => webMenu(e.currentTarget)} title={t("forge.open", { name: forgeName(project.remote) })} aria-label={t("forge.open", { name: forgeName(project.remote) })}>
            <Icon name="external" size={12} />
          </button>
        {/if}
      </div>
      <div class="sync-buttons">
        <button class="btn small" onclick={project.pull} disabled={!!project.gitBusy || !git.upstream} title={t("git.pullHint")}>
          <Icon name={project.gitBusy === "pull" ? "loader" : "pull"} size={13} class={project.gitBusy === "pull" ? "spin" : ""} />{t("git.pull")}
          {#if git.behind}<span class="count">{git.behind}</span>{/if}
        </button>
        <button class="btn small" class:primary={git.ahead > 0} onclick={project.push} disabled={!!project.gitBusy} title={git.upstream ? t("git.pushHint") : t("git.publishHint")}>
          <Icon name={project.gitBusy === "push" ? "loader" : "push"} size={13} class={project.gitBusy === "push" ? "spin" : ""} />{git.upstream ? t("git.push") : t("git.publish")}
          {#if git.ahead}<span class="count">{git.ahead}</span>{/if}
        </button>
      </div>
    </div>

    <div class="segmented tabs" role="tablist">
      <button class:on={project.gitTab === "changes"} onclick={() => setTab("changes")} role="tab" aria-selected={project.gitTab === "changes"}>
        {t("git.changes")}{#if git.changes.length}&nbsp;<span class="count">{git.changes.length}</span>{/if}{#if unseen}<span class="unseen on tab-dot"></span>{/if}
      </button>
      <button class:on={project.gitTab === "history"} onclick={() => setTab("history")} role="tab" aria-selected={project.gitTab === "history"}>{t("git.history")}</button>
      <button class:on={project.gitTab === "checkpoints"} onclick={() => setTab("checkpoints")} role="tab" aria-selected={project.gitTab === "checkpoints"} title={t("checkpoints.intro")}>
        {t("git.checkpoints")}
      </button>
    </div>

    {#if project.gitTab === "changes"}
      <div class="commit">
        <textarea
          class="field"
          rows="2"
          placeholder={t("git.messagePlaceholder")}
          bind:value={project.commitMessage}
          onkeydown={onKey}
          spellcheck="false"
        ></textarea>
        <button
          class="btn primary"
          onclick={() => project.commit()}
          disabled={!!project.gitBusy || !git.changes.length || !project.commitMessage.trim()}
          title={`${staged.length ? t("git.commitStagedHint") : t("git.commitHint")} (${mod}↵)`}
        >
          <Icon name={project.gitBusy === "commit" ? "loader" : "check"} size={13} class={project.gitBusy === "commit" ? "spin" : ""} />
          {staged.length ? t("git.commitStaged", { count: staged.length }) : t("git.commitAll")}
        </button>
      </div>
      <div class="list">
        {#if unseen}
          <div class="news">
            <span class="unseen on"></span>
            <span class="news-text">{t("unseen.count", { count: unseen })}</span>
            <button class="btn ghost tiny" onclick={() => project.markAllSeen()}>{t("unseen.markAll")}</button>
          </div>
        {/if}
        {#if staged.length}
          <div class="section">
            <span>{t("git.staged")}</span><span class="count-badge">{staged.length}</span>
            <button class="btn ghost tiny" onclick={() => project.unstage(staged.map((c) => c.path))}>{t("git.unstageAll")}</button>
          </div>
          {#each staged as c (c.path)}{@render changeRow(c, true)}{/each}
          {#if working.length}
            <div class="section">
              <span>{t("git.changes")}</span><span class="count-badge">{working.length}</span>
              <button class="btn ghost tiny" onclick={() => project.stage(working.map((c) => c.path))}>{t("git.stageAll")}</button>
            </div>
          {/if}
        {/if}
        {#each working as c (c.path)}
          {@render changeRow(c, false)}
        {:else}
          {#if !staged.length}<p class="empty-note clean"><Icon name="check" size={14} />{t("git.clean")}</p>{/if}
        {/each}
      </div>
    {:else if project.gitTab === "checkpoints"}
      <div class="cp-head">
        <label class="cp-auto" title={t("checkpoints.intro")}>
          <input type="checkbox" checked={settings.checkpoints} onchange={(e) => setCheckpoints(e.currentTarget.checked)} />
          {t("checkpoints.auto")}
        </label>
        <button class="icon-btn" onclick={() => project.takeCheckpoint("manual")} disabled={!!project.checkpointsOff} title={t("checkpoints.now")} aria-label={t("checkpoints.now")}>
          <Icon name="save" size={13} />
        </button>
        <button class="icon-btn" onclick={() => project.clearCheckpoints()} disabled={!project.checkpoints?.length} title={t("checkpoints.clear")} aria-label={t("checkpoints.clear")}>
          <Icon name="trash" size={13} />
        </button>
      </div>
      {#if !project.checkpointsOff}
        <button class="cp-usage" onclick={limitMenu} title={t("checkpoints.usageHint")}>
          <Icon name="database" size={12} />{usage}<Icon name="down" size={11} />
        </button>
      {/if}
      <div class="list">
        {#if project.checkpointsOff}
          <p class="empty-note">{project.checkpointsOff}</p>
        {:else if project.checkpoints === null}
          <p class="empty-note">{t("common.loading")}</p>
        {:else}
          {#each project.checkpoints as cp (cp.hash)}
            {@const open = project.expandedCommit === cp.hash}
            {@const when = moment(cp.date)}
            <div class="cp" class:open>
              <button class="cp-main" onclick={() => project.toggleCommitFiles(cp.hash)} aria-expanded={open} title={cp.hash}>
                <Icon name={REASON_ICON[cp.reason] ?? "save"} size={14} />
                <span class="cp-text">
                  {#if Date.parse(cp.date) > newSince}<span class="new">{t("unseen.new")}</span>{/if}
                  <span class="cp-title ellipsis">{t(`checkpoints.reason.${cp.reason}` as Key) ?? cp.reason}{cp.note ? ` · ${cp.note}` : ""}</span>
                  <span class="meta">{when} · {ago(cp.date, i18n.locale)} · {t("checkpoints.files", { count: cp.files })}</span>
                </span>
              </button>
              <button class="btn small" onclick={() => project.restoreCheckpoint(cp, when)} disabled={project.checkpointBusy} title={t("checkpoints.restoreHint")}>
                <Icon name="rotate-ccw" size={12} />{t("checkpoints.restore")}
              </button>
            </div>
            {#if open}<div class="cp-files files">{@render fileList(cp, false)}</div>{/if}
          {:else}
            <p class="empty-note">{t("checkpoints.none")}</p>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="scope">
        <Segmented
          stretch
          label={t("git.history")}
          value={settings.historyAll}
          onchange={setScope}
          options={[
            { value: false, label: t("git.thisBranch") },
            { value: true, label: t("git.allBranches") },
          ]}
        />
      </div>
      <div class="list">
        {#each project.log as c, i (c.hash)}
          {@const refs = refLabels(c.refs)}
          {@const open = project.expandedCommit === c.hash}
          <button class="commit-row" class:open onclick={() => project.toggleCommitFiles(c.hash)} oncontextmenu={(e) => commitMenu(e, c)} title={c.hash} aria-expanded={open}>
            {#if graph[i]}<GraphCell row={graph[i]} width={graphW} merge={c.parents.length > 1} head={refs.some((r) => r.current)} />{/if}
            <span class="commit-text">
              <span class="subject-line">
                {#each refs as r (r.kind + r.name)}
                  <span class="ref ellipsis {r.kind}" class:current={r.current} title={t(`git.ref.${r.kind}`)}>{r.name}</span>
                {/each}
                <span class="subject ellipsis">{c.subject}</span>
              </span>
              <span class="meta">
                {#if c.local}<span class="local" title={t("git.notPushed")}><Icon name="arrow-up" size={10} /></span>{/if}
                <span class="mono">{c.short}</span> · {c.author} · {ago(c.date, i18n.locale)}
              </span>
            </span>
          </button>
          {#if open}
            <div class="commit-files">
              <!-- The graph's lines carry on beside the list. -->
              <div class="lanes" style:width="{graphW}px" aria-hidden="true">
                {#each lanesAfter(i) as l (l.lane)}<span style:left="{laneX(l.lane) - 1}px" style:background={laneColor(l.color)}></span>{/each}
              </div>
              <div class="files">{@render fileList(c)}</div>
            </div>
          {/if}
        {:else}
          <p class="empty-note">{t("git.noCommits")}</p>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .git {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    min-width: 0;
  }
  .norepo {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 32px 16px;
    color: var(--muted);
    text-align: center;
    font-size: 12px;
  }
  .norepo p {
    margin: 0;
  }
  .sync {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--line);
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    min-width: 0;
  }
  .branch :global(svg) {
    color: var(--accent);
    flex: none;
  }
  .upstream {
    color: var(--muted);
    font-size: 11px;
  }
  .sync-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .small {
    height: 26px;
  }
  /* Commits to pull or push, as a badge beside the label. */
  .count {
    min-width: 17px;
    height: 17px;
    margin-left: 2px;
    padding: 0 5px;
    display: inline-grid;
    place-items: center;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 14%, transparent);
    font: 600 10px/1 var(--mono);
  }
  .btn.primary .count {
    background: rgb(0 0 0 / 0.2);
  }
  .tabs {
    margin: 10px 12px 0;
    display: flex;
  }
  .tabs button {
    flex: 1;
    padding: 5px 8px;
  }
  .commit {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .commit textarea {
    resize: none;
    font-family: var(--sans);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 12px;
  }
  .change {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 26px;
    padding-right: 10px;
    font-size: 12.5px;
  }
  .change-main {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 4px 0 12px;
    border: none;
    background: none;
    text-align: left;
  }
  /* Revert, stage and unstage show on the row under the pointer (or with the keyboard). */
  .row-actions {
    display: flex;
    flex: none;
    opacity: 0;
  }
  .change:hover .row-actions,
  .row-actions:focus-within {
    opacity: 1;
  }
  .row-actions .icon-btn {
    width: 22px;
    height: 22px;
    color: var(--muted);
  }
  .row-actions .icon-btn:hover:not(:disabled) {
    color: var(--text);
  }
  .section {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 4px 14px;
    font: 500 10px/1.4 var(--mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .section .btn {
    margin-left: auto;
    text-transform: none;
    letter-spacing: 0;
    font-family: var(--sans);
  }
  .change:hover,
  .commit-row:hover {
    background: var(--panel-2);
  }
  .name {
    flex: none;
    max-width: 60%;
  }
  .dir {
    flex: 1;
    color: var(--muted);
    font-size: 11px;
    direction: rtl;
    text-align: left;
  }
  .clean {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .clean :global(svg) {
    color: var(--ok);
  }
  .commit-row {
    width: 100%;
    height: 46px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px 0 2px;
    border: none;
    background: none;
    text-align: left;
  }
  .commit-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .subject-line {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .ref {
    flex: none;
    max-width: 120px;
    padding: 0 5px;
    border-radius: 4px;
    border: 1px solid var(--line);
    font: 10px/16px var(--mono);
    color: var(--muted);
  }
  .ref.branch {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--text);
  }
  .ref.current,
  .ref.head {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    border-color: var(--accent);
    color: var(--text);
    font-weight: 600;
  }
  .ref.tag {
    border-style: dashed;
  }
  .commit-row.open {
    background: var(--panel-2);
  }
  .commit-files {
    display: flex;
    /* No vertical padding here: the lane strip must reach both rows. */
    padding-left: 2px;
  }
  .lanes {
    position: relative;
    flex: none;
    margin-right: 4px;
  }
  .lanes span {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    border-radius: 1px;
  }
  .files {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--line);
    padding: 2px 0 6px 4px;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 12px 0 6px;
    border: none;
    border-radius: 5px;
    background: none;
    font-size: 12px;
    text-align: left;
    min-width: 0;
  }
  .file:hover {
    background: var(--panel-2);
  }
  .git :global(.type-icon) {
    flex: none;
    color: var(--muted);
  }
  .file .name {
    flex: none;
    max-width: 60%;
  }
  .file .dir {
    flex: 1;
    font-size: 10.5px;
    color: var(--muted);
  }
  .lines {
    flex: none;
    font: 10.5px var(--mono);
    color: var(--muted);
  }
  .lines.add {
    color: var(--ok);
  }
  .lines.del {
    color: var(--danger);
  }
  .files-note {
    margin: 0;
    padding: 4px 6px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .full {
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 2px;
    padding: 3px 6px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    font-size: 11px;
  }
  .full:hover {
    color: var(--accent);
    background: var(--panel-2);
  }
  /* "Changed since you last looked". */
  .unseen {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .unseen.on {
    background: var(--accent);
  }
  .tab-dot {
    display: inline-block;
    margin-left: 5px;
    vertical-align: middle;
  }
  .news {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 14px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .news-text {
    flex: 1;
  }
  .new {
    margin-right: 6px;
    padding: 0 5px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
    font: 600 9.5px/15px var(--mono);
    text-transform: uppercase;
  }
  .cp-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 8px 4px 12px;
  }
  .cp-usage {
    flex: none;
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 0 0 4px 8px;
    padding: 2px 6px;
    border: none;
    border-radius: 5px;
    background: none;
    font: 11px var(--mono);
    color: var(--muted);
  }
  .cp-usage:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .cp-auto {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--muted);
  }
  .cp {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 10px;
  }
  .cp.open,
  .cp:hover {
    background: var(--panel-2);
  }
  .cp-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 4px 7px 12px;
    border: none;
    background: none;
    text-align: left;
  }
  .cp-main :global(svg) {
    flex: none;
    color: var(--accent);
  }
  .cp-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cp-title {
    font-size: 12.5px;
  }
  .cp .btn {
    flex: none;
    opacity: 0;
  }
  .cp:hover .btn,
  .cp.open .btn,
  .cp .btn:focus-visible {
    opacity: 1;
  }
  .cp-files {
    margin: 0 0 6px 30px;
  }
  .web,
  .tasks {
    width: 22px;
    height: 22px;
    color: var(--muted);
  }
  .tasks {
    margin-left: auto;
  }
  .scope {
    flex: none;
    margin: 8px 12px 4px;
  }
  .subject {
    flex: 1;
    font-size: 12.5px;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .local {
    display: inline-flex;
    color: var(--accent);
  }
</style>
