import { expect, test } from "bun:test";
import { layoutGraph, refLabels } from "./graph";

const c = (hash: string, ...parents: string[]) => ({ hash, parents });

test("a straight history stays in one lane", () => {
  const rows = layoutGraph([c("c", "b"), c("b", "a"), c("a")]);
  expect(rows.map((r) => r.col)).toEqual([0, 0, 0]);
  expect(rows[0].top).toEqual([]); // the tip starts the lane
  expect(rows[1].top).toEqual([{ from: 0, to: 0, color: 0 }]);
  expect(rows[2].bottom).toEqual([]); // the first commit ends it
  expect(rows.every((r) => r.width === 1)).toBe(true);
});

test("a merge opens a lane for the branch and closes it at the fork", () => {
  //  m      merge of b2 into a2
  //  |\
  //  a2 b2
  //  |  b1
  //  | /
  //  a1
  const rows = layoutGraph([c("m", "a2", "b2"), c("a2", "a1"), c("b2", "b1"), c("b1", "a1"), c("a1")]);
  const [m, a2, b2, b1, a1] = rows;
  expect(m.col).toBe(0);
  expect(m.bottom).toEqual([
    { from: 0, to: 0, color: 0 },
    { from: 0, to: 1, color: 1 },
  ]);
  expect(a2.col).toBe(0);
  expect(b2.col).toBe(1);
  expect(b1.col).toBe(1);
  // b1's first parent is already awaited in lane 0: it bends over to it.
  expect(b1.bottom).toContainEqual({ from: 1, to: 0, color: 1 });
  expect(a1.col).toBe(0);
  expect(a1.width).toBe(1);
});

test("two branch tips get their own lanes and colours", () => {
  const rows = layoutGraph([c("x", "base"), c("y", "base"), c("base")]);
  expect(rows[0].col).toBe(0);
  expect(rows[1].col).toBe(1);
  expect(rows[1].color).not.toBe(rows[0].color);
  // y joins x's lane at base.
  expect(rows[1].bottom).toContainEqual({ from: 1, to: 0, color: 1 });
  expect(rows[2].col).toBe(0);
});

test("ref labels tell branches, remotes and tags apart", () => {
  expect(refLabels(["refs/remotes/origin/feature/x", "tag: refs/tags/v0.2.0", "HEAD -> refs/heads/feature/x", "refs/remotes/origin/HEAD"])).toEqual([
    { name: "feature/x", kind: "branch", current: true },
    { name: "origin/feature/x", kind: "remote", current: false },
    { name: "v0.2.0", kind: "tag", current: false },
  ]);
  expect(refLabels(["HEAD"])).toEqual([{ name: "HEAD", kind: "head", current: true }]);
});

test("a merge whose first parent is awaited elsewhere keeps its dot's colour", () => {
  // tip's first parent is m's first parent too; m's lane is handed to its second parent.
  const rows = layoutGraph([c("tip", "base"), c("m", "base", "side"), c("side", "base"), c("base")]);
  const m = rows[1];
  expect(m.col).toBe(1);
  expect(m.top).toEqual([{ from: 0, to: 0, color: 0 }]);
  expect(m.color).toBe(1); // its own lane's colour, not the new branch's
  expect(m.bottom).toContainEqual({ from: 1, to: 0, color: 1 });
});
