import { describe, expect, test } from "bun:test";
import { hunkTexts, parseDiff, splitDiff, stats } from "./diff";
import { join, relative, baseName } from "./paths";
import { languageName } from "./languages";

const DIFF = `diff --git a/src/app.ts b/src/app.ts
index 1111111..2222222 100644
--- a/src/app.ts
+++ b/src/app.ts
@@ -1,3 +1,4 @@
 import x from "x";
-const a = 1;
+const a = 2;
+const b = 3;
 export { a };
`;

test("parses a unified diff with line numbers", () => {
  const rows = parseDiff(DIFF);
  expect(rows[0]).toEqual({ kind: "file", text: "src/app.ts" });
  const body = rows.filter((r) => ["add", "del", "ctx"].includes(r.kind));
  expect(body.map((r) => [r.kind, r.old, r.new])).toEqual([
    ["ctx", 1, 1],
    ["del", 2, undefined],
    ["add", undefined, 2],
    ["add", undefined, 3],
    ["ctx", 3, 4],
  ]);
  expect(stats(rows)).toEqual({ added: 2, removed: 1 });
});

test("splits a diff into side-by-side rows, pairing an edit's two versions", () => {
  const pairs = splitDiff(parseDiff(DIFF)).filter((r) => r.kind === "pair");
  expect(pairs.map((r) => (r.kind === "pair" ? [r.left?.kind, r.left?.no, r.right?.kind, r.right?.no] : []))).toEqual([
    ["ctx", 1, "ctx", 1],
    ["del", 2, "add", 2],
    [undefined, undefined, "add", 3],
    ["ctx", 3, "ctx", 4],
  ]);
  const edit = pairs[1];
  // "const a = 1;" → "const a = 2;": only the digit changed.
  expect(edit.kind === "pair" && edit.left?.changed).toEqual([10, 11]);
  expect(edit.kind === "pair" && edit.right?.changed).toEqual([10, 11]);
});

test("joins and relativizes paths on both systems", () => {
  expect(join("/home/a/proj", "src/x.ts")).toBe("/home/a/proj/src/x.ts");
  expect(join("C:\\code\\proj", "src/x.ts")).toBe("C:\\code\\proj\\src\\x.ts");
  expect(relative("/home/a/proj", "/home/a/proj/src/x.ts")).toBe("src/x.ts");
  expect(relative("C:\\code\\proj", "C:\\code\\proj\\src\\x.ts")).toBe("src/x.ts");
  expect(relative("/home/a/proj", "/home/a/project2/x.ts")).toBeNull();
  expect(baseName("C:\\code\\proj\\")).toBe("proj");
});

test("names languages from file names", () => {
  expect(languageName("App.svelte")).toBe("Svelte");
  expect(languageName("main.rs")).toBe("Rust");
  expect(languageName("Dockerfile")).toBe("Plain text");
  expect(languageName("tsconfig.json")).toBe("JSON");
});

test("an empty diff has no rows", () => {
  expect(parseDiff("")).toEqual([]);
  expect(parseDiff("\n")).toEqual([]);
});

test("parses a merge's combined diff", () => {
  const rows = parseDiff(`diff --cc f
index 4dc8328,2339517..3e42aa1
--- a/f
+++ b/f
@@@ -1,3 -1,3 +1,3 @@@
  one
- TWO main
 -TWO side
++TWO both
  three
`);
  expect(rows[0]).toEqual({ kind: "file", text: "f" });
  const body = rows.filter((r) => ["add", "del", "ctx"].includes(r.kind));
  expect(body.map((r) => [r.kind, r.text, r.old, r.new])).toEqual([
    ["ctx", "one", 1, 1],
    ["del", "TWO main", 2, undefined],
    ["del", "TWO side", undefined, undefined],
    ["add", "TWO both", undefined, 2],
    ["ctx", "three", 3, 3],
  ]);
  expect(stats(rows)).toEqual({ added: 1, removed: 2 });
});

describe("hunkTexts", () => {
  const head = "diff --git a/x.ts b/x.ts\nindex 1..2 100644\n--- a/x.ts\n+++ b/x.ts\n";
  test("each hunk as Git printed it, numbered in the rows", () => {
    const text = `${head}@@ -1,2 +1,2 @@\n-a\n+b\n c\n@@ -9 +9 @@\n-y\n+z\n`;
    expect(hunkTexts(text)).toEqual(["@@ -1,2 +1,2 @@\n-a\n+b\n c\n", "@@ -9 +9 @@\n-y\n+z\n"]);
    expect(parseDiff(text).filter((r) => r.kind === "hunk").map((r) => r.hunk)).toEqual([0, 1]);
  });
  test("none for a new, deleted or cut-off file", () => {
    expect(hunkTexts("diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1 @@\n+x\n")).toBeNull();
    expect(hunkTexts("diff --git a/n b/n\ndeleted file mode 100644\n--- a/n\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n")).toBeNull();
    expect(hunkTexts(`${head}@@ -1 +1 @@\n-a\n+b\n… (cut off)\n`)).toBeNull();
    expect(hunkTexts("")).toBeNull();
  });
});
