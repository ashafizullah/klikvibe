// The terminal's Prompts menu: the built-in prompts (in the app's language)
// and the user's own, until the user edits or removes one. From then on the
// whole list is theirs, kept as they wrote it.
import { t } from "./i18n/index.svelte";
import { persist, settings } from "./settings.svelte";

export const BUILT_IN = ["prompts.fixTests", "prompts.review", "prompts.explain", "prompts.commitMessage", "prompts.simpler"] as const;

export function promptList(): string[] {
  return settings.promptList ?? [...BUILT_IN.map((k) => t(k)), ...settings.prompts];
}

export function savePrompts(list: string[]) {
  settings.promptList = list;
  persist();
}

/** Built-in prompts no longer on the list (removed, or changed). */
export function missingBuiltIns(): string[] {
  const list = promptList();
  return BUILT_IN.map((k) => t(k)).filter((p) => !list.includes(p));
}

/** Puts the missing built-in prompts back at the top, keeping the user's. */
export function restoreBuiltIns() {
  savePrompts([...missingBuiltIns(), ...promptList()]);
}
