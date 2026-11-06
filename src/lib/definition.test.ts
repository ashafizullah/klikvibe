import { expect, test } from "bun:test";
import { componentFiles, importedFrom, moduleAt, rankDefinitions, resolveImport, rustModule } from "./definition";

const files = new Set([
  "src/lib/ipc.ts",
  "src/lib/i18n/index.svelte.ts",
  "src/components/Icon.svelte",
  "src/components/forms/index.ts",
  "app/models/__init__.py",
  "app/views.py",
  "app/utils.py",
  "src-tauri/src/main.rs",
  "src-tauri/src/git.rs",
  "src-tauri/src/mcp/mod.rs",
  "src/styles/base.css",
]);

test("imports point at project files", () => {
  expect(resolveImport("../lib/ipc", "src/components/Editor.svelte", files)).toBe("src/lib/ipc.ts");
  expect(resolveImport("./Icon.svelte", "src/components/Editor.svelte", files)).toBe("src/components/Icon.svelte");
  expect(resolveImport("./forms", "src/components/Editor.svelte", files)).toBe("src/components/forms/index.ts");
  expect(resolveImport("$lib/ipc", "src/routes/+page.svelte", files)).toBe("src/lib/ipc.ts");
  expect(resolveImport("@/lib/ipc", "src/App.svelte", files)).toBe("src/lib/ipc.ts");
  expect(resolveImport("./styles/base.css", "src/app.css", files)).toBe("src/styles/base.css");
  expect(resolveImport(".utils", "app/views.py", files)).toBe("app/utils.py");
  expect(resolveImport("app.models", "app/views.py", files)).toBe("app/models/__init__.py");
  expect(resolveImport("svelte", "src/App.svelte", files)).toBeNull();
  expect(resolveImport("../../../x", "src/App.svelte", files)).toBeNull();
});

test("the module under the cursor", () => {
  const line = `import { ipc } from "../lib/ipc";`;
  expect(moduleAt(line, line.indexOf("lib"))).toBe("../lib/ipc");
  expect(moduleAt(line, line.indexOf("ipc"))).toBeNull();
  expect(moduleAt(`const s = "hello";`, 12)).toBeNull();
  expect(moduleAt("from .utils import helper", 7)).toBe(".utils");
});

test("Rust modules", () => {
  expect(rustModule("mod git;", "git", "src-tauri/src/main.rs", files)).toBe("src-tauri/src/git.rs");
  expect(rustModule("pub mod mcp;", "mcp", "src-tauri/src/main.rs", files)).toBe("src-tauri/src/mcp/mod.rs");
  expect(rustModule("use crate::git;", "git", "src-tauri/src/main.rs", files)).toBeNull();
});

test("where a name was imported from", () => {
  const doc = `import Icon from "./Icon.svelte";\nimport { ipc, type Entry as E } from "../lib/ipc";\nfrom .utils import helper, other\n`;
  expect(importedFrom(doc, "Icon")).toBe("./Icon.svelte");
  expect(importedFrom(doc, "ipc")).toBe("../lib/ipc");
  expect(importedFrom(doc, "helper")).toBe(".utils");
  expect(importedFrom(doc, "ip")).toBeNull();
});

test("components by file name, and ranking", () => {
  expect(componentFiles("Icon", files)).toEqual(["src/components/Icon.svelte"]);
  expect(componentFiles("icon", files)).toEqual([]);
  const m = (path: string, line: number) => ({ path, line, text: "", start: 0, end: 0 });
  const found = [m("b.ts", 3), m("a.ts", 9), m("lib.ts", 1)];
  expect(rankDefinitions(found, "a.ts", "lib.ts")).toEqual({ list: [m("lib.ts", 1), m("a.ts", 9), m("b.ts", 3)], sure: true });
  expect(rankDefinitions([m("b.ts", 3), m("c.ts", 1)], "a.ts", null).sure).toBe(false);
  expect(rankDefinitions([m("b.ts", 3)], "a.ts", null).sure).toBe(true);
});
