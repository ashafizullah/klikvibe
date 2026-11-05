// Unified diffs (`git diff`, `git show`) split into rows for display.

export type RowKind = "add" | "del" | "ctx" | "hunk" | "file" | "meta";

export interface Row {
  kind: RowKind;
  text: string;
  /** Line numbers in the old and new file, where they apply. */
  old?: number;
  new?: number;
}

export interface DiffStats {
  added: number;
  removed: number;
}

export function parseDiff(text: string): Row[] {
  const rows: Row[] = [];
  let oldNo = 0;
  let newNo = 0;
  let inHunk = false;
  for (const line of text.replace(/\n$/, "").split("\n")) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (hunk) {
      oldNo = Number(hunk[1]);
      newNo = Number(hunk[2]);
      inHunk = true;
      rows.push({ kind: "hunk", text: line });
    } else if (line.startsWith("diff --git ")) {
      inHunk = false;
      rows.push({ kind: "file", text: line.replace(/^diff --git a\/(.*) b\/.*$/, "$1") });
    } else if (!inHunk) {
      rows.push({ kind: "meta", text: line });
    } else if (line.startsWith("+")) {
      rows.push({ kind: "add", text: line.slice(1), new: newNo++ });
    } else if (line.startsWith("-")) {
      rows.push({ kind: "del", text: line.slice(1), old: oldNo++ });
    } else if (line.startsWith("\\")) {
      rows.push({ kind: "meta", text: line });
    } else {
      rows.push({ kind: "ctx", text: line.slice(1), old: oldNo++, new: newNo++ });
    }
  }
  return rows;
}

export function stats(rows: Row[]): DiffStats {
  let added = 0;
  let removed = 0;
  for (const r of rows) {
    if (r.kind === "add") added++;
    else if (r.kind === "del") removed++;
  }
  return { added, removed };
}

/** One side of a side-by-side row. */
export interface Half {
  kind: "add" | "del" | "ctx";
  no: number;
  text: string;
  /** The changed part of an edited line (between an unchanged start and end), to highlight. */
  changed?: [number, number];
}

export type SplitRow = { kind: "file" | "meta" | "hunk"; text: string } | { kind: "pair"; left?: Half; right?: Half };

/** The part of `a` that differs from `b`, after their common start and before their common end. */
function changedPart(a: string, b: string): [number, number] {
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start++;
  let end = 0;
  while (end < a.length - start && end < b.length - start && a[a.length - 1 - end] === b[b.length - 1 - end]) end++;
  return [start, a.length - end];
}

/** Unified rows as side-by-side rows: removed lines on the left, added on the right, an edit's two versions on one row. */
export function splitDiff(rows: Row[]): SplitRow[] {
  const out: SplitRow[] = [];
  let dels: Row[] = [];
  let adds: Row[] = [];
  const flush = () => {
    for (let i = 0; i < Math.max(dels.length, adds.length); i++) {
      const d = dels[i];
      const a = adds[i];
      const left: Half | undefined = d && { kind: "del", no: d.old!, text: d.text };
      const right: Half | undefined = a && { kind: "add", no: a.new!, text: a.text };
      if (left && right) {
        left.changed = changedPart(d.text, a.text);
        right.changed = changedPart(a.text, d.text);
      }
      out.push({ kind: "pair", left, right });
    }
    dels = [];
    adds = [];
  };
  for (const r of rows) {
    if (r.kind === "del") {
      // A removal after additions starts a new edit.
      if (adds.length) flush();
      dels.push(r);
    } else if (r.kind === "add") adds.push(r);
    else {
      flush();
      if (r.kind === "ctx") out.push({ kind: "pair", left: { kind: "ctx", no: r.old!, text: r.text }, right: { kind: "ctx", no: r.new!, text: r.text } });
      else out.push({ kind: r.kind, text: r.text });
    }
  }
  flush();
  return out;
}
