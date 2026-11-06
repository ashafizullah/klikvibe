import { describe, expect, test } from "bun:test";

import { grid, type PanelSettings } from "./grid";

const base: PanelSettings = { layout: "classic", showTree: true, showTreeColumns: true, showCode: true, showGit: true, showTerminal: true, treeWidth: 250, gitWidth: 320, terminalWidth: 560 };

describe("grid", () => {
  const g = (over: Partial<PanelSettings>) => grid({ ...base, ...over });

  test("classic: terminal under the code, a strip when hidden", () => {
    expect(g({}).areas).toBe(`"tree rztree editor rzgit git" "tree rztree term rzgit git"`);
    expect(g({ showTerminal: false }).placed.has("term")).toBe(true);
    expect(g({}).terminalFills).toBe(false);
  });

  test("classic without code: the terminal takes the column", () => {
    const x = g({ showCode: false });
    expect(x.areas).toBe(`"term rzgit git"`);
    expect(x.terminalFills).toBe(true);
    expect(x.placed.has("tree")).toBe(false);
  });

  test("classic with no file open: the terminal goes up to the top, the tree stays", () => {
    const x = grid(base, true);
    expect(x.areas).toBe(`"tree rztree term rzgit git"`);
    expect(x.terminalFills).toBe(true);
    expect(x.placed.has("editor")).toBe(false);
    // A hidden terminal leaves the code area (and its welcome card) as it was.
    expect(grid({ ...base, showTerminal: false }, true).placed.has("editor")).toBe(true);
    // The columns layout has the terminal in a column of its own already.
    expect(grid({ ...base, layout: "columns" }, true).placed.has("editor")).toBe(true);
  });

  test("columns: Terminal | Code | Git, the code fills", () => {
    const x = g({ layout: "columns" });
    expect(x.areas).toBe(`"term rzterm tree rztree editor rzgit git"`);
    expect(x.columns).toBe("560px auto 250px auto minmax(0, 1fr) auto 320px");
  });

  test("columns keep their own file-tree choice", () => {
    expect(g({ layout: "columns", showTreeColumns: false }).placed.has("tree")).toBe(false);
    expect(g({ layout: "classic", showTreeColumns: false }).placed.has("tree")).toBe(true);
  });

  test("columns without code: the terminal fills", () => {
    const x = g({ layout: "columns", showCode: false });
    expect(x.columns).toBe("minmax(0, 1fr) auto 320px");
    expect(g({ layout: "columns", showCode: false, showTerminal: false }).columns).toBe("minmax(0, 1fr)");
  });
});
