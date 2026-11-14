// Changed files grouped under the project's top folders ("backend", "frontend"), for the Git panel.

export interface FolderGroup<T> {
  /** The top folder, or "" for files at the project's top level. */
  folder: string;
  items: T[];
}

/** `items` by their path's first folder: top-level files first, then the folders A to Z. */
export function byTopFolder<T extends { path: string }>(
  items: T[],
): FolderGroup<T>[] {
  const groups = new Map<string, T[]>();
  for (const item of items) {
    const folder = topFolder(item.path);
    const list = groups.get(folder);
    if (list) list.push(item);
    else groups.set(folder, [item]);
  }
  return [...groups]
    .sort(([a], [b]) => (a === "" ? -1 : b === "" ? 1 : a.localeCompare(b)))
    .map(([folder, items]) => ({ folder, items }));
}

/** The first folder in a Git path (forward slashes), or "" for a top-level file. */
export function topFolder(path: string): string {
  const i = path.indexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}
