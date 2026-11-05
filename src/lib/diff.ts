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
