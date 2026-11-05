// Which panels show and how they are laid out: Terminal, Code (editor and
// file tree) and Git can each be hidden, but never all three at once.
import { persist, settings, type Layout } from "./settings.svelte";
import { treeKey } from "./grid";

export type PanelKey = "showTerminal" | "showCode" | "showGit" | "showTree";
const MAIN: PanelKey[] = ["showTerminal", "showCode", "showGit"];

/** Whether hiding this panel would leave nothing on screen. */
export function isLastPanel(key: PanelKey): boolean {
  return MAIN.includes(key) && settings[key] && MAIN.filter((k) => settings[k]).length === 1;
}

export function togglePanel(key: PanelKey) {
  if (isLastPanel(key)) return;
  if (key === "showTree") {
    // The file tree lives in the code column: showing it shows the code too.
    const tree = treeKey(settings);
    if (!settings.showCode) settings.showCode = settings[tree] = true;
    else settings[tree] = !settings[tree];
  } else settings[key] = !settings[key];
  persist();
}

export function setLayout(layout: Layout) {
  settings.layout = layout;
  persist();
}
