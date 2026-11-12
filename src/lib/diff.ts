// Unified diffs (`git diff`, `git show`) split into rows for display.

export type RowKind = "add" | "del" | "ctx" | "hunk" | "file" | "meta";

export interface Row {
  kind: RowKind;
  text: string;
  /** A hunk's place among the diff's hunks (its "@@" row). */
  hunk?: number;
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
  // Nothing changed: no rows (not one empty line).
  if (!text.trim()) return rows;
  let oldNo = 0;
  let newNo = 0;
  let inHunk = false;
  // The columns of +, - or space before each line: one, or one per parent in a merge's
  // combined diff ("@@@ -1,3 -1,3 +1,5 @@@", after "diff --cc").
  let cols = 1;
  let hunks = 0;
  for (const line of text.replace(/\n$/, "").split("\n")) {
    const hunk = /^(@@+) -(\d+)(?:,\d+)? (?:-\d+(?:,\d+)? )*\+(\d+)(?:,\d+)? \1/.exec(line);
    const file = /^diff --(?:git a\/(.*) b\/.*|(?:cc|combined) (.*))$/.exec(line);
    if (hunk) {
      cols = hunk[1].length - 1;
      oldNo = Number(hunk[2]);
      newNo = Number(hunk[3]);
      inHunk = true;
      rows.push({ kind: "hunk", text: line, hunk: hunks++ });
    } else if (file) {
      inHunk = false;
      rows.push({ kind: "file", text: file[1] ?? file[2] });
    } else if (!inHunk) {
      rows.push({ kind: "meta", text: line });
    } else if (line.startsWith("\\")) {
      rows.push({ kind: "meta", text: line });
    } else {
      // In a combined diff the old line numbers are the first parent's.
      const marks = line.slice(0, cols);
      const body = line.slice(cols);
      if (marks.includes("+")) rows.push({ kind: "add", text: body, new: newNo++ });
      // A line only the other parent had isn't one of the first parent's: it has no number.
      else if (marks.includes("-")) rows.push({ kind: "del", text: body, old: marks[0] === "-" ? oldNo++ : undefined });
      else rows.push({ kind: "ctx", text: body, old: oldNo++, new: newNo++ });
    }
  }
  return rows;
}

/**
 * The hunks of a one-file diff as Git printed them (each from its "@@" line), to hand
 * one back to Git (git::hunk); null when they can't be: a new, binary or cut-off file.
 */
export function hunkTexts(text: string): string[] | null {
  if (!text.trim() || /^(new file mode|deleted file mode|Binary files|--- \/dev\/null|\+\+\+ \/dev\/null)/m.test(text) || text.includes("\n… (cut off)\n")) return null;
  if ((text.match(/^diff --git /gm) ?? []).length !== 1) return null;
  const hunks: string[] = [];
  for (const line of text.split(/(?<=\n)/)) {
    if (line.startsWith("@@")) hunks.push(line);
    else if (hunks.length) hunks[hunks.length - 1] += line;
  }
  return hunks;
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

export type SplitRow = { kind: "file" | "meta" | "hunk"; text: string; hunk?: number } | { kind: "pair"; left?: Half; right?: Half };

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
      else out.push({ kind: r.kind, text: r.text, hunk: r.hunk });
    }
  }
  flush();
  return out;
}
