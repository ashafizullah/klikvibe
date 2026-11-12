// Path helpers that work for both macOS/Linux and Windows roots.

export function sepOf(root: string): string {
  return root.includes("\\") && !root.includes("/") ? "\\" : "/";
}

/** `rel` (Git's forward-slash form) under `root`. */
export function join(root: string, rel: string): string {
  const sep = sepOf(root);
  return root.replace(/[\\/]+$/, "") + sep + rel.split("/").join(sep);
}

/** `path` relative to `root` with forward slashes, or null when outside it. */
export function relative(root: string, path: string): string | null {
  const base = root.replace(/[\\/]+$/, "");
  if (path === base) return "";
  if (!path.startsWith(base) || !/[\\/]/.test(path.charAt(base.length))) return null;
  return path.slice(base.length + 1).split("\\").join("/");
}

/** Like `relative`, for a path someone else wrote (an agent): either slash, and on Windows any
 *  letter case ("c:/users/a/proj/x.ts" is under "C:\\Users\\a\\proj"). */
export function relativeLoose(root: string, path: string): string | null {
  const windows = /^[a-zA-Z]:/.test(root) || root.includes("\\");
  const norm = (p: string) => {
    const s = p.replace(/\\/g, "/").replace(/\/+$/, "");
    return windows ? s.toLowerCase() : s;
  };
  const base = norm(root);
  const full = norm(path);
  if (full === base) return "";
  if (!full.startsWith(`${base}/`)) return null;
  // The rest in the caller's own spelling (only the root's case is ignored).
  return path.replace(/\\/g, "/").replace(/\/+$/, "").slice(base.length + 1);
}

export function baseName(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path;
}

export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}

/** The drag of a file or folder from the file tree (its absolute path), dropped on a terminal. */
export const TREE_DRAG = "application/x-klikvibe-path";

/** `path` as one word for a shell: quoted when it has spaces or characters the shell would read. */
export function shellQuote(path: string, windows = false): string {
  if ((windows ? /^[\w@%+=:,./\\-]+$/ : /^[\w@%+=:,./-]+$/).test(path)) return path;
  return windows ? `"${path}"` : `'${path.replace(/'/g, `'\\''`)}'`;
}
