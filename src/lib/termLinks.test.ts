import { expect, test } from "bun:test";
import { findLinks, projectPath } from "./termLinks";

const paths = (s: string) => findLinks(s).filter((l) => l.kind === "path").map((l) => [l.target, l.line, l.column]);

test("paths as compilers, tests and agents print them", () => {
  expect(paths("error: src/lib/x.ts:42:7 - Type 'a' is not assignable")).toEqual([["src/lib/x.ts", 42, 7]]);
  expect(paths("  --> src/main.rs:10:5")).toEqual([["src/main.rs", 10, 5]]);
  expect(paths("app.ts(12,5): error TS2304")).toEqual([["app.ts", 12, 5]]);
  expect(paths("Updated ./src/App.svelte and ../README.md")).toEqual([
    ["./src/App.svelte", undefined, undefined],
    ["../README.md", undefined, undefined],
  ]);
  expect(paths("/Users/me/proj/src/a.svelte:3")).toEqual([["/Users/me/proj/src/a.svelte", 3, undefined]]);
});

test("versions, numbers and words are not paths", () => {
  expect(paths("KlikVibe 0.1.0 is ready, 2.5 seconds")).toEqual([]);
  expect(paths("bun v1.3.4")).toEqual([]);
});

test("web addresses, without the sentence's punctuation", () => {
  const [l] = findLinks("Local:   http://localhost:5173/, see https://example.com/a?b=1.");
  expect(l).toEqual({ kind: "url", start: 9, end: 31, target: "http://localhost:5173/," .slice(0, -1) });
  expect(findLinks("see https://example.com/a?b=1.").map((x) => x.target)).toEqual(["https://example.com/a?b=1"]);
});

test("printed paths become project paths", () => {
  expect(projectPath("./src/a.ts", "/p")).toBe("src/a.ts");
  expect(projectPath("/p/src/a.ts", "/p/")).toBe("src/a.ts");
  expect(projectPath("/elsewhere/a.ts", "/p")).toBeNull();
  expect(projectPath("../a.ts", "/p")).toBeNull();
  expect(projectPath("~/p/x.rs", "/home/me/p", "/home/me")).toBe("x.rs");
  expect(projectPath("C:\\code\\p\\src\\a.ts", "C:\\code\\p")).toBe("src/a.ts");
});
