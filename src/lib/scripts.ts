// The project's runnable scripts (package.json, Cargo) and dev server
// addresses found in terminal output, for the terminal's Scripts menu and
// the preview tab.

export type PackageManager = "bun" | "pnpm" | "yarn" | "npm";

export interface Script {
  name: string;
  /** What runs in the terminal. */
  command: string;
  /** What the script itself does (package.json), shown as a hint. */
  detail?: string;
}

/** The package manager the lockfile says, npm without one. */
export function packageManager(rootNames: string[]): PackageManager {
  const has = (n: string) => rootNames.includes(n);
  if (has("bun.lock") || has("bun.lockb")) return "bun";
  if (has("pnpm-lock.yaml")) return "pnpm";
  if (has("yarn.lock")) return "yarn";
  return "npm";
}

/** package.json's scripts as commands for `pm`; none when the file isn't valid JSON. */
export function packageScripts(json: string, pm: PackageManager): Script[] {
  let scripts: unknown;
  try {
    scripts = JSON.parse(json)?.scripts;
  } catch {
    return [];
  }
  if (!scripts || typeof scripts !== "object") return [];
  return Object.entries(scripts as Record<string, unknown>)
    .filter(([, v]) => typeof v === "string")
    .map(([name, detail]) => ({ name, command: `${pm} run ${name}`, detail: detail as string }));
}

export const CARGO_SCRIPTS: Script[] = [
  { name: "cargo run", command: "cargo run" },
  { name: "cargo test", command: "cargo test" },
  { name: "cargo build", command: "cargo build" },
];

// eslint-disable-next-line no-control-regex
const ANSI = /\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07/g;
const LOCAL = /\bhttps?:\/\/(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\]):\d{2,5}(?:\/[^\s'"<>)\]]*)?/;

/** A local dev server's address in some terminal output ("Local: http://localhost:5173/"), with 0.0.0.0 made reachable. */
export function devServerUrl(output: string): string | null {
  const m = LOCAL.exec(output.replace(ANSI, ""));
  if (!m) return null;
  return m[0].replace(/[.,;:]+$/, "").replace("//0.0.0.0:", "//localhost:").replace("//[::1]:", "//localhost:");
}
