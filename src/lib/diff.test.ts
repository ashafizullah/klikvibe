import { expect, test } from "bun:test";
import { parseDiff, stats } from "./diff";
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
