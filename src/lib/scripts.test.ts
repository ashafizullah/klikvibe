import { expect, test } from "bun:test";
import { devServerUrl, packageManager, packageScripts } from "./scripts";

test("the lockfile picks the package manager", () => {
  expect(packageManager(["package.json", "bun.lock"])).toBe("bun");
  expect(packageManager(["pnpm-lock.yaml"])).toBe("pnpm");
  expect(packageManager(["yarn.lock"])).toBe("yarn");
  expect(packageManager(["package.json"])).toBe("npm");
});

test("package.json scripts become commands", () => {
  const json = JSON.stringify({ scripts: { dev: "vite", test: "bun test", odd: 3 } });
  expect(packageScripts(json, "bun")).toEqual([
    { name: "dev", command: "bun run dev", detail: "vite" },
    { name: "test", command: "bun run test", detail: "bun test" },
  ]);
  expect(packageScripts("{not json", "npm")).toEqual([]);
  expect(packageScripts("{}", "npm")).toEqual([]);
});

test("dev server addresses in terminal output", () => {
  expect(devServerUrl("  \x1b[32m➜\x1b[39m  \x1b[1mLocal\x1b[22m:   \x1b[36mhttp://localhost:\x1b[1m5173\x1b[22m/\x1b[39m")).toBe("http://localhost:5173/");
  expect(devServerUrl("ready - started server on 0.0.0.0:3000, url: http://0.0.0.0:3000")).toBe("http://localhost:3000");
  expect(devServerUrl("Listening on http://127.0.0.1:8000/api.")).toBe("http://127.0.0.1:8000/api");
  expect(devServerUrl("see https://example.com:443/x")).toBeNull();
  expect(devServerUrl("nothing here")).toBeNull();
});
