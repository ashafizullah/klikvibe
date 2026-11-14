import { expect, test } from "bun:test";
import { byTopFolder, topFolder } from "./groups";

test("groups by the first folder, top-level files first", () => {
  const files = [
    "frontend/src/App.svelte",
    "README.md",
    "backend/main.go",
    "frontend/package.json",
    "backend/api/user.go",
    ".env",
  ].map((path) => ({ path }));
  expect(
    byTopFolder(files).map((g) => [g.folder, g.items.map((i) => i.path)]),
  ).toEqual([
    ["", ["README.md", ".env"]],
    ["backend", ["backend/main.go", "backend/api/user.go"]],
    ["frontend", ["frontend/src/App.svelte", "frontend/package.json"]],
  ]);
});

test("top folder of a path", () => {
  expect(topFolder("a.ts")).toBe("");
  expect(topFolder("src/a.ts")).toBe("src");
  expect(topFolder("src/lib/a.ts")).toBe("src");
});
