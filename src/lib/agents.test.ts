import { describe, expect, test } from "bun:test";
import { AGENTS, agentById, clientName } from "./agents";

const info = { url: "http://127.0.0.1:47825/mcp", token: "secret123" };

describe("agents", () => {
  test("ids are unique and every setup carries the URL and token", () => {
    expect(new Set(AGENTS.map((a) => a.id)).size).toBe(AGENTS.length);
    for (const a of AGENTS) {
      const s = a.mcp(info);
      if (s.kind === "none") continue;
      expect(s.text).toContain(info.url);
      expect(s.text).toContain("Bearer secret123");
      if (s.kind === "file" && s.file.endsWith(".json")) expect(() => JSON.parse(s.text)).not.toThrow();
    }
  });

  test("unknown ids fall back to the first agent", () => {
    expect(agentById("nope").id).toBe("claude");
  });

  test("client names read well", () => {
    expect(clientName("claude-code 2.1.0")).toBe("Claude Code");
    expect(clientName("codex-mcp-client 0.50.0")).toBe("Codex");
    expect(clientName("opencode 1.0")).toBe("OpenCode");
    expect(clientName(null)).toBe("MCP");
    expect(clientName("mystery-agent 1")).toBe("mystery-agent");
  });
});
