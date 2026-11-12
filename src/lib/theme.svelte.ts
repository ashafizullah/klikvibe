// Dark or light: tokens.css has both palettes, picked by <html data-theme>. The
// terminal draws its own colours, so it gets them from here (terminalTheme).
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ITheme } from "@xterm/xterm";
import { persist, settings, type ThemeChoice } from "./settings.svelte";

const light = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: light)") : null;
const os = $state({ light: !!light?.matches });
light?.addEventListener("change", (e) => (os.light = e.matches));

/** The colours on screen now. */
export function currentTheme(): "dark" | "light" {
  return settings.theme === "system" ? (os.light ? "light" : "dark") : settings.theme;
}

export function setTheme(choice: ThemeChoice) {
  settings.theme = choice;
  persist();
}

/** Follows the setting (and the computer's, with "system") from now on; call once at start. */
export function initTheme() {
  document.documentElement.dataset.theme = currentTheme();
  $effect.root(() => {
    $effect(() => {
      document.documentElement.dataset.theme = currentTheme();
    });
    // The window's own parts (the traffic lights, the background behind the page). Left to the
    // computer for "system", or the page would only ever hear the forced one.
    $effect(() => {
      const choice = settings.theme;
      getCurrentWindow()
        .setTheme(choice === "system" ? null : choice)
        .catch(() => {});
    });
  });
}

const DARK: ITheme = { background: "#15130f", foreground: "#ede6d8", cursor: "#f0a83a", cursorAccent: "#15130f", selectionBackground: "#f0a83a44" };

/** xterm's defaults are for a dark background: on paper, colours that still read (yellow and white darker). */
const LIGHT: ITheme = {
  background: "#f6f2e9",
  foreground: "#2a251d",
  cursor: "#b36a06",
  cursorAccent: "#f6f2e9",
  selectionBackground: "#b36a0633",
  black: "#2a251d",
  red: "#c4323a",
  green: "#3f7a1a",
  yellow: "#8a6400",
  blue: "#2a5fb0",
  magenta: "#a03c9a",
  cyan: "#1b7f8c",
  white: "#7a705f",
  brightBlack: "#6f6656",
  brightRed: "#d9473f",
  brightGreen: "#4f8f24",
  brightYellow: "#9c7400",
  brightBlue: "#3a72c8",
  brightMagenta: "#b54db0",
  brightCyan: "#2491a0",
  brightWhite: "#a89d88",
};

export function terminalTheme(): ITheme {
  return currentTheme() === "light" ? LIGHT : DARK;
}

/** The highlight of the matches of a find in the terminal. */
export function findColors() {
  return currentTheme() === "light"
    ? { matchBackground: "#f3d79a", activeMatchBackground: "#e8a93a" }
    : { matchBackground: "#5a4520", activeMatchBackground: "#b07418" };
}
