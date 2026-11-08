import { expect, test } from "bun:test";
import { fromDisk, toDisk } from "./eol";

test("a file with CRLF line ends is saved with them", () => {
  const disk = "a\r\nb\r\n";
  const { text, eol } = fromDisk(disk);
  expect(text).toBe("a\nb\n");
  expect(eol).toBe("\r\n");
  expect(toDisk(text + "c\n", eol)).toBe("a\r\nb\r\nc\r\n");
});

test("LF files stay LF, and a mostly-LF file is not turned into CRLF", () => {
  expect(fromDisk("a\nb\n")).toEqual({ text: "a\nb\n", eol: "\n" });
  expect(fromDisk("a\nb\nc\r\nd\n").eol).toBe("\n");
  expect(toDisk("a\nb\n", "\n")).toBe("a\nb\n");
});
