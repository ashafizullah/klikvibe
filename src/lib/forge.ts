// Web links for a repository's remote: the repo, a branch, a commit, a file
// (at a line or lines) and a new pull or merge request. GitHub, GitLab
// (gitlab.com or self-hosted, when the host names it) and Bitbucket; any
// other host gets the repository link only.

export type Forge = "github" | "gitlab" | "bitbucket" | "other";

export interface Remote {
  forge: Forge;
  /** https://host/owner/repo */
  base: string;
}

/** The web address behind a remote URL (SSH, scp-like or HTTPS); null when it isn't one. */
export function parseRemote(url: string): Remote | null {
  const u = url.trim();
  let host: string;
  let path: string;
  // git@github.com:owner/repo.git
  const scp = /^[\w.-]+@([\w.-]+):(?!\/\/)(.+)$/.exec(u);
  if (scp) {
    host = scp[1];
    path = scp[2];
  } else {
    // ssh://git@host:2222/owner/repo.git, https://user@host/owner/repo.git
    const m = /^(?:ssh|https?|git):\/\/(?:[^@/]+@)?([^/:]+)(?::\d+)?\/(.+)$/.exec(u);
    if (!m) return null;
    host = m[1];
    path = m[2];
  }
  path = path.replace(/\/+$/, "").replace(/\.git$/, "");
  if (!path.includes("/")) return null;
  const h = host.toLowerCase();
  const forge: Forge = h.includes("github") ? "github" : h.includes("gitlab") ? "gitlab" : h.includes("bitbucket") ? "bitbucket" : "other";
  return { forge, base: `https://${host}/${path}` };
}

const NAME: Record<Forge, string> = { github: "GitHub", gitlab: "GitLab", bitbucket: "Bitbucket", other: "web" };
export const forgeName = (r: Remote) => NAME[r.forge];

/** "Pull request" on GitHub and Bitbucket, "merge request" on GitLab. */
export const isMergeRequest = (r: Remote) => r.forge === "gitlab";

const enc = (path: string) => path.split("/").map(encodeURIComponent).join("/");

export function branchUrl(r: Remote, branch: string): string | null {
  if (r.forge === "github") return `${r.base}/tree/${enc(branch)}`;
  if (r.forge === "gitlab") return `${r.base}/-/tree/${enc(branch)}`;
  if (r.forge === "bitbucket") return `${r.base}/branch/${enc(branch)}`;
  return null;
}

export function commitUrl(r: Remote, hash: string): string | null {
  if (r.forge === "github") return `${r.base}/commit/${hash}`;
  if (r.forge === "gitlab") return `${r.base}/-/commit/${hash}`;
  if (r.forge === "bitbucket") return `${r.base}/commits/${hash}`;
  return null;
}

/** A file at `ref` (a branch or commit), optionally from line `from` to `to`. */
export function fileUrl(r: Remote, ref: string, path: string, from?: number, to?: number): string | null {
  const lines = (a: string, sep: string) => (from ? `#${a}${from}${to && to !== from ? `${sep}${to}` : ""}` : "");
  if (r.forge === "github") return `${r.base}/blob/${enc(ref)}/${enc(path)}${lines("L", "-L")}`;
  if (r.forge === "gitlab") return `${r.base}/-/blob/${enc(ref)}/${enc(path)}${lines("L", "-")}`;
  if (r.forge === "bitbucket") return `${r.base}/src/${enc(ref)}/${enc(path)}${lines("lines-", ":")}`;
  return null;
}

/** The page that starts a pull (merge) request from `branch`. */
export function newRequestUrl(r: Remote, branch: string): string | null {
  if (r.forge === "github") return `${r.base}/compare/${enc(branch)}?expand=1`;
  if (r.forge === "gitlab") return `${r.base}/-/merge_requests/new?merge_request%5Bsource_branch%5D=${encodeURIComponent(branch)}`;
  if (r.forge === "bitbucket") return `${r.base}/pull-requests/new?source=${encodeURIComponent(branch)}`;
  return null;
}
