// Finding files by name in the Files panel. A query is words separated by
// spaces; each word is either text the path must contain, or a glob with
// `*`, `**` and `?`:
//   button          paths containing "button"
//   *.ts            files ending in .ts (a glob without "/" matches the file name)
//   src/**/*.svelte  a glob with "/" matches the whole path from the root
//   !*.test.ts      a leading "!" leaves matches out
// Case is ignored.

export interface NameMatch {
  path: string;
  name: string;
  dir: string;
  /** The part of the name to highlight, if a word was found in it. */
  mark: [number, number] | null;
}

const isGlob = (word: string) => /[*?]/.test(word);

/** A glob as a whole-string, case-insensitive regular expression. */
export function globToRegExp(glob: string): RegExp {
  let re = "";
  const g = glob.replace(/^\.?\//, "");
  for (let i = 0; i < g.length; i++) {
    const c = g[i];
    if (c === "*" && g[i + 1] === "*") {
      // "**/" also matches no folder at all.
      re += g[i + 2] === "/" ? "(?:.*/)?" : ".*";
      i += g[i + 2] === "/" ? 2 : 1;
    } else if (c === "*") re += "[^/]*";
    else if (c === "?") re += "[^/]";
    else re += c.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${re}$`, "i");
}

interface Word {
  not: boolean;
  test: (path: string, name: string) => boolean;
  text: string | null;
}

function parse(query: string): Word[] {
  return query
    .split(/[\s,]+/)
    .filter(Boolean)
    .map((raw) => {
      const not = raw.startsWith("!") && raw.length > 1;
      const word = not ? raw.slice(1) : raw;
      if (isGlob(word)) {
        const re = globToRegExp(word);
        const whole = word.includes("/");
        return { not, text: null, test: (path, name) => re.test(whole ? path : name) };
      }
      const lower = word.toLowerCase();
      return { not, text: lower, test: (path) => path.toLowerCase().includes(lower) };
    });
}

/** The files matching `query`, best first, at most `limit`. */
export function matchNames(files: string[], query: string, limit = 200): NameMatch[] {
  const words = parse(query);
  const yes = words.filter((w) => !w.not);
  const no = words.filter((w) => w.not);
  const globs = yes.filter((w) => w.text === null);
  const texts = yes.filter((w) => w.text !== null);
  if (!yes.length) return [];

  const found: (NameMatch & { score: number })[] = [];
  for (const path of files) {
    const slash = path.lastIndexOf("/");
    const name = path.slice(slash + 1);
    if (texts.some((w) => !w.test(path, name))) continue;
    if (globs.length && !globs.some((w) => w.test(path, name))) continue;
    if (no.some((w) => w.test(path, name))) continue;
    const first = texts[0]?.text ?? null;
    const at = first === null ? -1 : name.toLowerCase().indexOf(first);
    // A name that starts with the first word, then one that contains it, then the rest; shorter paths first.
    const score = (first === null ? 2 : at === 0 ? 0 : at > 0 ? 1 : 2) * 10_000 + path.length;
    found.push({ path, name, dir: slash < 0 ? "" : path.slice(0, slash), mark: at >= 0 ? [at, at + first!.length] : null, score });
  }
  found.sort((a, b) => a.score - b.score || a.path.localeCompare(b.path));
  return found.slice(0, limit).map(({ score: _score, ...m }) => m);
}
