import { expect, test } from "bun:test";
import { globToRegExp, matchNames } from "./search";

const FILES = [
  "README.md",
  "src/App.svelte",
  "src/components/Button.svelte",
  "src/components/ButtonGroup.svelte",
  "src/lib/button.test.ts",
  "src/lib/button.ts",
  "src/lib/paths.ts",
  "src-tauri/src/git.rs",
];
const paths = (q: string) => matchNames(FILES, q).map((m) => m.path);

test("globs", () => {
  expect(globToRegExp("*.ts").test("paths.ts")).toBe(true);
  expect(globToRegExp("*.ts").test("src/paths.ts")).toBe(false);
  expect(globToRegExp("src/**/*.svelte").test("src/App.svelte")).toBe(true);
  expect(globToRegExp("src/**/*.svelte").test("src/components/Button.svelte")).toBe(true);
  expect(globToRegExp("git.r?").test("git.rs")).toBe(true);
  expect(globToRegExp("a.b").test("axb")).toBe(false);
});

test("text finds paths, names starting with it first", () => {
  expect(paths("button")).toEqual([
    "src/lib/button.ts",
    "src/lib/button.test.ts",
    "src/components/Button.svelte",
    "src/components/ButtonGroup.svelte",
  ]);
  expect(matchNames(FILES, "group")[0].mark).toEqual([6, 11]);
});

test("a glob without a slash matches names, with one the whole path", () => {
  expect(paths("*.rs")).toEqual(["src-tauri/src/git.rs"]);
  expect(paths("src/**/*.svelte")).toEqual(["src/App.svelte", "src/components/Button.svelte", "src/components/ButtonGroup.svelte"]);
  expect(paths("src/*.svelte")).toEqual(["src/App.svelte"]);
});

test("words combine, and ! leaves out", () => {
  expect(paths("*.ts !*.test.ts")).toEqual(["src/lib/paths.ts", "src/lib/button.ts"]);
  expect(paths("lib *.ts")).toEqual(["src/lib/paths.ts", "src/lib/button.ts", "src/lib/button.test.ts"]);
  expect(paths("!*.ts")).toEqual([]);
});
