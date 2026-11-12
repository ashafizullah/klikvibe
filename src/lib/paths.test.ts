import { expect, test } from "bun:test";
import { join, relativeLoose, shellQuote } from "./paths";

test("an agent's Windows path matches the project whatever its slashes and letter case", () => {
  const root = "C:\\Users\\a\\proj";
  expect(relativeLoose(root, "C:\\Users\\a\\proj\\src\\x.ts")).toBe("src/x.ts");
  expect(relativeLoose(root, "c:/users/a/proj/src/x.ts")).toBe("src/x.ts");
  expect(relativeLoose(root, "C:/Users/a/proj/")).toBe("");
  expect(relativeLoose(root, "C:\\Users\\a\\project\\x.ts")).toBeNull();
  expect(join(root, "src/x.ts")).toBe("C:\\Users\\a\\proj\\src\\x.ts");
});

test("on macOS and Linux the case still counts", () => {
  expect(relativeLoose("/Users/a/proj", "/Users/a/proj/src/x.ts")).toBe("src/x.ts");
  expect(relativeLoose("/Users/a/proj", "/users/a/proj/src/x.ts")).toBeNull();
  expect(relativeLoose("/Users/a/proj", "/Users/a/proj2/x.ts")).toBeNull();
});

test("a dropped path is one word for the shell", () => {
  expect(shellQuote("src/lib/x.ts")).toBe("src/lib/x.ts");
  expect(shellQuote("My Docs/a b.md")).toBe("'My Docs/a b.md'");
  expect(shellQuote("it's.txt")).toBe(`'it'\\''s.txt'`);
  expect(shellQuote("$HOME.txt")).toBe("'$HOME.txt'");
  expect(shellQuote("My Docs\\a.md", true)).toBe('"My Docs\\a.md"');
  expect(shellQuote("src\\x.ts", true)).toBe("src\\x.ts");
});
