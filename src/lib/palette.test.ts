import { expect, test } from "bun:test";
import { rank, score } from "./palette";

const items = [
  { id: "files", text: "View: Show or hide Files" },
  { id: "terminal", text: "Terminal: New terminal" },
  { id: "push", text: "Git: Push" },
  { id: "pull", text: "Git: Pull" },
];

test("every word typed has to be found, whole or as letters in order", () => {
  expect(score("push", "Git: Push")).not.toBeNull();
  expect(score("git psh", "Git: Push")).not.toBeNull();
  expect(score("git commit", "Git: Push")).toBeNull();
  expect(score("", "anything")).toBe(0);
});

test("a whole word beats letters in order, and the start of a word beats the middle", () => {
  expect(score("term", "Terminal: New terminal")!).toBeGreaterThan(score("trml", "Terminal: New terminal")!);
  expect(score("push", "Git: Push")!).toBeGreaterThan(score("ush", "Git: Push")!);
});

test("best first; with nothing typed, the ones used last come first", () => {
  expect(rank(items, "pu").map((x) => x.id)).toEqual(["push", "pull"]);
  expect(rank(items, "pull").map((x) => x.id)).toEqual(["pull"]);
  expect(rank(items, "", ["pull", "gone"]).map((x) => x.id)).toEqual(["pull", "files", "terminal", "push"]);
});
