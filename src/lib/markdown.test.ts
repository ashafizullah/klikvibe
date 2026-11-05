import { expect, test } from "bun:test";
import { renderMarkdown } from "./markdown";

test("renders lists, code and emphasis, and escapes HTML", () => {
  expect(renderMarkdown("- Open `omp`\n- **Fast**")).toBe("<ul><li>Open <code>omp</code></li><li><strong>Fast</strong></li></ul>");
  expect(renderMarkdown("<img src=x onerror=alert(1)>")).toBe("<p>&lt;img src=x onerror=alert(1)&gt;</p>");
});
