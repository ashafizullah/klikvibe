// The commit graph next to the history: which lane each commit's dot sits
// in, and the lines that pass, branch and merge around it. Commits come
// newest first with children before parents (git log --date-order).

export interface Segment {
  /** Lane at the start (top edge, or the dot for the lower half). */
  from: number;
  /** Lane at the end (the dot, or the bottom edge). */
  to: number;
  /** Lane colour index, in the order lanes appeared. */
  color: number;
}

export interface GraphRow {
  /** The commit's lane. */
  col: number;
  color: number;
  /** Lines from the row's top edge down to the dot's height. */
  top: Segment[];
  /** Lines from the dot's height down to the row's bottom edge. */
  bottom: Segment[];
  /** Lanes this row draws in (its width). */
  width: number;
}

export function layoutGraph(commits: { hash: string; parents: string[] }[]): GraphRow[] {
  // The commit each lane is waiting for, and its colour.
  const lanes: (string | null)[] = [];
  const colors: number[] = [];
  let nextColor = 0;
  const free = () => {
    const i = lanes.indexOf(null);
    return i < 0 ? lanes.length : i;
  };

  return commits.map((c) => {
    const top: Segment[] = [];
    const bottom: Segment[] = [];
    let col = lanes.indexOf(c.hash);
    if (col < 0) {
      // Nothing below points here yet: a branch tip starts a lane.
      col = free();
      colors[col] = nextColor++;
    } else top.push({ from: col, to: col, color: colors[col] });
    lanes[col] = c.hash;

    // Other lanes pass by, or end here when they were waiting for this commit too.
    const passing: number[] = [];
    lanes.forEach((h, i) => {
      if (i === col || h === null) return;
      if (h === c.hash) {
        top.push({ from: i, to: col, color: colors[i] });
        lanes[i] = null;
      } else {
        top.push({ from: i, to: i, color: colors[i] });
        passing.push(i);
      }
    });

    // The first parent continues this lane, unless another lane already waits for it.
    const [first, ...others] = c.parents;
    const waiting = first === undefined ? -1 : lanes.indexOf(first);
    if (first === undefined) lanes[col] = null;
    else if (waiting >= 0 && waiting !== col) {
      bottom.push({ from: col, to: waiting, color: colors[col] });
      lanes[col] = null;
    } else {
      lanes[col] = first;
      bottom.push({ from: col, to: col, color: colors[col] });
    }
    // A merge: its other parents join their lanes, or start new ones.
    for (const p of others) {
      let i = lanes.indexOf(p);
      if (i < 0) {
        i = free();
        lanes[i] = p;
        colors[i] = nextColor++;
      }
      bottom.push({ from: col, to: i, color: colors[i] });
    }
    for (const i of passing) if (lanes[i] !== null) bottom.push({ from: i, to: i, color: colors[i] });

    while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop();
    const width = Math.max(col, ...top.map((s) => Math.max(s.from, s.to)), ...bottom.map((s) => Math.max(s.from, s.to))) + 1;
    return { col, color: colors[col], top, bottom, width };
  });
}

export type RefKind = "head" | "branch" | "remote" | "tag";
export interface RefLabel {
  name: string;
  kind: RefKind;
  /** The branch checked out (HEAD points to it). */
  current: boolean;
}

/** Git's full decorations as labels; remote HEAD pointers ("origin/HEAD") are left out. */
export function refLabels(refs: string[]): RefLabel[] {
  const out: RefLabel[] = [];
  for (const raw of refs) {
    const current = raw.startsWith("HEAD -> ");
    const ref = current ? raw.slice("HEAD -> ".length) : raw;
    if (ref === "HEAD") out.push({ name: "HEAD", kind: "head", current: true });
    else if (ref.startsWith("tag: refs/tags/")) out.push({ name: ref.slice("tag: refs/tags/".length), kind: "tag", current: false });
    else if (ref.startsWith("refs/heads/")) out.push({ name: ref.slice("refs/heads/".length), kind: "branch", current });
    else if (ref.startsWith("refs/remotes/") && !ref.endsWith("/HEAD")) out.push({ name: ref.slice("refs/remotes/".length), kind: "remote", current: false });
  }
  // The checked-out branch first, then other branches, remotes, tags.
  const order: Record<RefKind, number> = { head: 0, branch: 1, remote: 2, tag: 3 };
  return out.sort((a, b) => Number(b.current) - Number(a.current) || order[a.kind] - order[b.kind]);
}
