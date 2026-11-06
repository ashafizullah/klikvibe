// The CSS grid for the panels: pure, so it can be tested without the settings store.
import type { Layout } from "./settings.svelte";

export interface PanelSettings {
  layout: Layout;
  showTree: boolean;
  showTreeColumns: boolean;
  showCode: boolean;
  showGit: boolean;
  showTerminal: boolean;
  treeWidth: number;
  gitWidth: number;
  terminalWidth: number;
}

/** Each layout remembers its own file-tree choice. */
export const treeKey = (s: Pick<PanelSettings, "layout">) => (s.layout === "columns" ? "showTreeColumns" : "showTree");
export const treeShown = (s: PanelSettings) => s[treeKey(s)];

export interface Grid {
  columns: string;
  rows: string;
  areas: string;
  /** Which elements are placed; the others are hidden (but stay mounted). */
  placed: Set<string>;
  /** The terminal fills its area instead of having its own height. */
  terminalFills: boolean;
}

/**
 * The CSS grid for the current layout and panels. `noFiles`: no tab is open,
 * so in the classic layout a shown terminal takes the code's place, full height,
 * until a file opens (the code's welcome card shows only with the terminal hidden).
 */
export function grid(s: PanelSettings, noFiles = false): Grid {
  const term = s.showTerminal;
  const emptyCode = s.layout === "classic" && s.showCode && noFiles && term;
  const code = s.showCode && !emptyCode;
  const git = s.showGit;
  // The file tree stays with an empty code area: it is where a file gets opened.
  const tree = (code || emptyCode) && treeShown(s);
  // One column takes the remaining width: the code, else the terminal, else Git.
  const fill = code ? "editor" : term ? "term" : "git";
  const cols: string[] = [];
  const top: string[] = [];
  const bottom: string[] = [];
  const add = (width: string, a: string, b = a) => (cols.push(width), top.push(a), bottom.push(b));
  const flex = "minmax(0, 1fr)";

  if (s.layout === "columns") {
    if (term) {
      add(fill === "term" ? flex : `${s.terminalWidth}px`, "term");
      // Between Terminal and Code; with no code, Git's own resizer sits next to the terminal.
      if (code) add("auto", "rzterm");
    }
    if (tree) add(`${s.treeWidth}px`, "tree"), add("auto", "rztree");
    if (code) add(flex, "editor");
  } else {
    if (tree) add(`${s.treeWidth}px`, "tree"), add("auto", "rztree");
    // The terminal sits under the code (a strip when hidden), or takes the whole column without it.
    if (code) add(flex, "editor", "term");
    else if (term) add(flex, "term");
  }
  if (git) {
    if (code || term) add("auto", "rzgit");
    add(fill === "git" ? flex : `${s.gitWidth}px`, "git");
  }

  const twoRows = s.layout === "classic" && code;
  const areas = (twoRows ? [top, bottom] : [top]).map((r) => `"${r.join(" ")}"`).join(" ");
  return {
    columns: cols.join(" "),
    rows: twoRows ? "minmax(0, 1fr) auto" : "minmax(0, 1fr)",
    areas,
    placed: new Set(top.concat(twoRows ? bottom : [])),
    terminalFills: s.layout === "columns" || !code,
  };
}
