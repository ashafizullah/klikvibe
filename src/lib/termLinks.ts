// Clickable things in terminal output: web addresses, and file paths with an
// optional line and column, the way compilers, test runners and agents print
// them:
//   src/lib/x.ts:42:7    --> src/main.rs:10:5    ./app.ts    app.ts(12,5)
//   /Users/me/proj/src/a.svelte:3
// Any other word may name a file or folder too (store-assets, src/lib,
// Makefile): those are "loose" candidates.
// Paths are only candidates here; the terminal links the ones that exist.

export interface TermLink {
  kind: "url" | "path";
  /** Character range in the line, end exclusive. */
  start: number;
  end: number;
  /** The address, or the path as printed (without line and column). */
  target: string;
  line?: number;
  column?: number;
  /** A plain word that may name a file or folder; only linked when it exists. */
  loose?: boolean;
}

const URL_RE = /\bhttps?:\/\/[^\s"'<>`)\]]+/g;
// A path has a folder part or an extension; then ":line[:col]" or "(line,col)".
const PATH_RE = /(?:~|\.{1,2})?\/?(?:[\w@.+-]+\/)*[\w@+-][\w@.+-]*\.[A-Za-z0-9]+(?::(\d+)(?::(\d+))?|\((\d+),(\d+)\))?/g;
// An absolute folder (or a file without an extension): /tmp/x/card/, ~/Projects/app.
const ABS_RE = /(?:~\/|\/)(?:[\w@.+-]+\/)*[\w@.+-]+\/?/g;
const WORD_RE = /[\w@+-][\w@.+-]*(?:\/[\w@.+-]+)*\/?/g;

export function findLinks(text: string): TermLink[] {
  const links: TermLink[] = [];
  for (const m of text.matchAll(URL_RE)) {
    // Trailing punctuation belongs to the sentence, not the address.
    const url = m[0].replace(/[.,;:!?]+$/, "");
    links.push({ kind: "url", start: m.index, end: m.index + url.length, target: url });
  }
  const taken = (i: number) => links.some((l) => i >= l.start && i < l.end);
  for (const m of text.matchAll(PATH_RE)) {
    const start = m.index;
    if (taken(start)) continue;
    // Not the middle of a longer word or path.
    if (start > 0 && /[\w@.\/-]/.test(text[start - 1])) continue;
    const full = m[0];
    const cut = full.search(/:\d|\(\d/);
    const path = cut < 0 ? full : full.slice(0, cut);
    // Skip numbers and versions such as "1.2", "0.1.0" or "v1.3.4".
    if (/^v?[\d.]+$/i.test(path)) continue;
    const line = Number(m[1] ?? m[3]) || undefined;
    const column = Number(m[2] ?? m[4]) || undefined;
    links.push({ kind: "path", start, end: start + full.length, target: path, line, column });
  }
  for (const m of text.matchAll(ABS_RE)) {
    const start = m.index;
    if (taken(start) || (start > 0 && /[\w@.\/:-]/.test(text[start - 1]))) continue;
    const path = m[0].replace(/[./]+$/, "");
    if (path.split("/").filter(Boolean).length < 2) continue;
    links.push({ kind: "path", start, end: start + path.length, target: path });
  }
  for (const m of text.matchAll(WORD_RE)) {
    const start = m.index;
    if (taken(start) || (start > 0 && /[\w@.\/-]/.test(text[start - 1]))) continue;
    // "folder/" and the sentence's full stop aren't part of the name.
    const word = m[0].replace(/[./]+$/, "");
    if (word.length < 2 || /^v?[\d.]+$/i.test(word)) continue;
    links.push({ kind: "path", start, end: start + word.length, target: word, loose: true });
  }
  return links.sort((a, b) => a.start - b.start);
}

/** `printed` as a path relative to `root` (forward slashes), or null when it points outside. */
export function projectPath(printed: string, root: string, home?: string): string | null {
  let p = printed.replace(/\\/g, "/");
  if (p.startsWith("~/") && home) p = home.replace(/\/+$/, "") + p.slice(1);
  const base = root.replace(/\\/g, "/").replace(/\/+$/, "");
  if (p.startsWith("/") || /^[A-Za-z]:\//.test(p)) {
    return p.startsWith(base + "/") ? p.slice(base.length + 1) : null;
  }
  const parts: string[] = [];
  for (const part of p.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (!parts.length) return null;
      parts.pop();
    } else parts.push(part);
  }
  return parts.join("/") || null;
}
