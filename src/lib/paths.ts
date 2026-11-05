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

export function baseName(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path;
}

export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}
