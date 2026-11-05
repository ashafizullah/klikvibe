import { describe, expect, test } from "bun:test";
import { AGENTS, agentById, agentIn, clientName, terminalName } from "./agents";

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

test("finds the agent in a foreground command line", () => {
  expect(agentIn("claude")).toBe("Claude Code");
  expect(agentIn("node /usr/local/bin/claude --resume")).toBe("Claude Code");
  expect(agentIn("/Users/me/.cargo/bin/codex exec")).toBe("Codex");
  expect(agentIn("cursor-agent")).toBe("Cursor CLI");
  expect(agentIn("-zsh")).toBeNull();
  expect(agentIn("vim claude.md")).toBeNull();
  expect(agentIn(null)).toBeNull();
});

test("names a terminal after what runs in it", () => {
  expect(terminalName("node /usr/local/bin/claude")).toBe("Claude Code");
  expect(terminalName("npm run dev")).toBe("npm run dev");
  expect(terminalName("node /proj/node_modules/.bin/vite --port 3000")).toBe("vite --port 3000");
  expect(terminalName("/opt/homebrew/bin/cargo watch -x test")).toBe("cargo watch -x");
  expect(terminalName("python3 manage.py runserver")).toBe("manage.py runserver");
  expect(terminalName("-zsh")).toBeNull();
  expect(terminalName("/bin/bash")).toBeNull();
  expect(terminalName("bash deploy.sh")).toBe("bash deploy.sh");
  expect(terminalName(null)).toBeNull();
  expect(terminalName("some-really-long-command-name --with flags")!.length).toBeLessThanOrEqual(24);
});
