import { expect, test } from "bun:test";
import { join, relativeLoose } from "./paths";

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
