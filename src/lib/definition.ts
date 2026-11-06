// Go to definition without a language server: the file an import points to,
// and lines that look like they define a name (found in Rust, files.rs).
// Pure helpers; CodeView wires them to ⌘-click and F12.

import type { TextMatch } from "./ipc";

const SCRIPT_EXTS = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "svelte", "vue", "d.ts"];
const RESOLVE_EXTS = [...SCRIPT_EXTS, "json", "css", "scss", "less", "py", "rs", "go"];

const dirOf = (path: string) => (path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");

/** "a/b/../c/./d" → "a/c/d"; null when it climbs out of the project. */
function normalize(path: string): string | null {
  const parts: string[] = [];
  for (const part of path.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      if (!parts.length) return null;
      parts.pop();
    } else parts.push(part);
  }
  return parts.join("/");
}

/** The first of `base` as is, with an extension, or as a folder's index file, that exists. */
function firstExisting(base: string, files: Set<string>): string | null {
  if (files.has(base)) return base;
  for (const ext of RESOLVE_EXTS) if (files.has(`${base}.${ext}`)) return `${base}.${ext}`;
  for (const ext of SCRIPT_EXTS) if (files.has(`${base}/index.${ext}`)) return `${base}/index.${ext}`;
  if (files.has(`${base}/__init__.py`)) return `${base}/__init__.py`;
  if (files.has(`${base}/mod.rs`)) return `${base}/mod.rs`;
  return null;
}

/**
 * The project file an import's module points to, from the file `from` (both
 * relative to the project root): "./x", "../x", "$lib/x" and "@/x" (SvelteKit and
 * Vite aliases for src/lib and src), "/x" from the root, and Python's dotted
 * modules (".models", "app.models"). Null for packages (react, node:fs) and misses.
 */
export function resolveImport(spec: string, from: string, files: Set<string>): string | null {
  const here = dirOf(from);
  let base: string | null = null;
  if (spec.startsWith("./") || spec.startsWith("../")) base = normalize(`${here}/${spec}`);
  else if (spec.startsWith("$lib/")) base = normalize(`src/lib/${spec.slice(5)}`);
  else if (spec.startsWith("@/") || spec.startsWith("~/")) base = normalize(`src/${spec.slice(2)}`);
  else if (spec.startsWith("/")) base = normalize(spec);
  else if (from.endsWith(".py") && /^\.*[\w.]+$/.test(spec)) {
    // ".models" from the same folder, "..models" from the one above, "app.models" from the root.
    const dots = spec.match(/^\.*/)![0].length;
    let dir = dots ? here : "";
    for (let i = 1; i < dots; i++) dir = dirOf(dir);
    base = normalize(`${dir}/${spec.slice(dots).replace(/\./g, "/")}`);
  }
  return base === null ? null : firstExisting(base, files);
}

/** The module inside the quotes at `column` of an import-like line ("from", "import", "require", "@import"). */
export function moduleAt(line: string, column: number): string | null {
  if (!/\b(?:import|from|require|export)\b|@import|@use/.test(line)) return null;
  for (const m of line.matchAll(/(["'`])([^"'`]+)\1/g)) {
    if (column > m.index && column <= m.index + m[0].length - 1) return m[2];
  }
  // Python: "from .models import X" / "import app.models" (no quotes).
  const py = /^\s*(?:from\s+(\.*[\w.]*)\s+import|import\s+([\w.]+))/.exec(line);
  const mod = py?.[1] ?? py?.[2];
  if (mod) {
    const at = line.indexOf(mod);
    if (column >= at && column <= at + mod.length) return mod;
  }
  return null;
}

/** Rust's "mod name;": the file it means, next to `from` (or in its own folder for a non-root file). */
export function rustModule(line: string, name: string, from: string, files: Set<string>): string | null {
  if (!new RegExp(`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?mod\\s+${name}\\s*;`).test(line)) return null;
  const file = from.slice(from.lastIndexOf("/") + 1);
  const root = ["main.rs", "lib.rs", "mod.rs"].includes(file);
  const dir = root ? dirOf(from) : `${dirOf(from)}/${file.replace(/\.rs$/, "")}`.replace(/^\//, "");
  return firstExisting(`${dir ? `${dir}/` : ""}${name}`, files);
}

/** The module the current file imports `name` from ("import { name } from './x'", "import Name from './Name.svelte'"). */
export function importedFrom(doc: string, name: string): string | null {
  const word = new RegExp(`(?:^|[^\\w$])${name.replace(/\$/g, "\\$")}(?:[^\\w$]|$)`);
  for (const m of doc.matchAll(/import\s+(?:type\s+)?([\s\S]*?)\s+from\s+["']([^"']+)["']/g)) {
    if (word.test(m[1])) return m[2];
  }
  for (const m of doc.matchAll(/^\s*from\s+(\.*[\w.]*)\s+import\s+(.+)$/gm)) {
    if (word.test(m[2])) return m[1];
  }
  return null;
}

/** Component files named like `name` (Button → Button.svelte, Button.tsx), for JSX and Svelte tags. */
export function componentFiles(name: string, files: Iterable<string>): string[] {
  if (!/^[A-Z]/.test(name)) return [];
  const want = new Set(["svelte", "vue", "tsx", "jsx"].map((ext) => `${name}.${ext}`));
  return [...files].filter((f) => want.has(f.slice(f.lastIndexOf("/") + 1)));
}

/**
 * The definitions to offer, best first: in the file the name was imported from,
 * then in the current file, then the rest; `sure` when the first is the one to
 * jump to without asking.
 */
export function rankDefinitions(found: TextMatch[], current: string, imported: string | null): { list: TextMatch[]; sure: boolean } {
  const rank = (m: TextMatch) => (m.path === imported ? 0 : m.path === current ? 1 : 2);
  const list = [...found].sort((a, b) => rank(a) - rank(b) || a.path.localeCompare(b.path) || a.line - b.line);
  const top = list.filter((m) => rank(m) === rank(list[0]));
  return { list, sure: list.length === 1 || (top.length === 1 && rank(list[0]) < 2) };
}
