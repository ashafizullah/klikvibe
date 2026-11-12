// The command palette's search: which commands match what was typed, best first.

/**
 * How well `query` matches `text`, higher is better; null when it doesn't.
 * Each word typed has to be found: whole (best at the start of a word) or as
 * letters in order ("tgl term" finds "Toggle terminal").
 */
export function score(query: string, text: string): number | null {
  const hay = text.toLowerCase();
  let total = 0;
  for (const word of query.toLowerCase().split(/\s+/).filter(Boolean)) {
    const at = hay.indexOf(word);
    if (at >= 0) {
      total += /[\s:/(·._-]/.test(hay[at - 1] ?? " ") ? 3 : 2;
      continue;
    }
    let from = 0;
    let last = -1;
    let gaps = 0;
    for (const ch of word) {
      const i = hay.indexOf(ch, from);
      if (i < 0) return null;
      if (last >= 0 && i > last + 1) gaps++;
      last = i;
      from = i + 1;
    }
    total += 1 / (1 + gaps);
  }
  return total;
}

/**
 * `items` that match `query`, best first (a tie keeps their order). With nothing typed:
 * all of them, the ones in `recent` (ids, last used first) at the top.
 */
export function rank<T extends { id: string; text: string }>(items: T[], query: string, recent: string[] = []): T[] {
  if (!query.trim()) {
    const used = recent.map((id) => items.find((x) => x.id === id)).filter((x): x is T => !!x);
    return [...used, ...items.filter((x) => !used.includes(x))];
  }
  return items
    .map((item, i) => ({ item, i, s: score(query, item.text) }))
    .filter((x) => x.s !== null)
    .sort((a, b) => b.s! - a.s! || a.i - b.i)
    .map((x) => x.item);
}
