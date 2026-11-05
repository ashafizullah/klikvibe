// Syntax highlighting per file type. Each language loads only when a file
// of that type is first opened, which keeps the app small to start.
import type { LanguageSupport, StreamLanguage } from "@codemirror/language";

type Loader = () => Promise<LanguageSupport | StreamLanguage<unknown>>;

interface Lang {
  name: string;
  load: Loader;
}

const js = (opts: { jsx?: boolean; typescript?: boolean }): Loader => async () => (await import("@codemirror/lang-javascript")).javascript(opts);

const LANGS: Record<string, Lang> = {
  js: { name: "JavaScript", load: js({}) },
  mjs: { name: "JavaScript", load: js({}) },
  cjs: { name: "JavaScript", load: js({}) },
  jsx: { name: "JavaScript (JSX)", load: js({ jsx: true }) },
  ts: { name: "TypeScript", load: js({ typescript: true }) },
  mts: { name: "TypeScript", load: js({ typescript: true }) },
  cts: { name: "TypeScript", load: js({ typescript: true }) },
  tsx: { name: "TypeScript (TSX)", load: js({ jsx: true, typescript: true }) },
  // Svelte and Vue files are mostly HTML with script and style blocks.
  svelte: { name: "Svelte", load: async () => (await import("@codemirror/lang-html")).html() },
  vue: { name: "Vue", load: async () => (await import("@codemirror/lang-html")).html() },
  html: { name: "HTML", load: async () => (await import("@codemirror/lang-html")).html() },
  htm: { name: "HTML", load: async () => (await import("@codemirror/lang-html")).html() },
  css: { name: "CSS", load: async () => (await import("@codemirror/lang-css")).css() },
  scss: { name: "SCSS", load: async () => (await import("@codemirror/lang-css")).css() },
  json: { name: "JSON", load: async () => (await import("@codemirror/lang-json")).json() },
  jsonc: { name: "JSON", load: async () => (await import("@codemirror/lang-json")).json() },
  md: { name: "Markdown", load: async () => (await import("@codemirror/lang-markdown")).markdown() },
  markdown: { name: "Markdown", load: async () => (await import("@codemirror/lang-markdown")).markdown() },
  py: { name: "Python", load: async () => (await import("@codemirror/lang-python")).python() },
  rs: { name: "Rust", load: async () => (await import("@codemirror/lang-rust")).rust() },
  yml: { name: "YAML", load: async () => (await import("@codemirror/lang-yaml")).yaml() },
  yaml: { name: "YAML", load: async () => (await import("@codemirror/lang-yaml")).yaml() },
  sql: { name: "SQL", load: async () => (await import("@codemirror/lang-sql")).sql() },
  php: { name: "PHP", load: async () => (await import("@codemirror/lang-php")).php() },
  go: { name: "Go", load: async () => (await import("@codemirror/lang-go")).go() },
  java: { name: "Java", load: async () => (await import("@codemirror/lang-java")).java() },
  kt: { name: "Kotlin", load: async () => (await import("@codemirror/lang-java")).java() },
  c: { name: "C", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
  h: { name: "C", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
  cpp: { name: "C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
  hpp: { name: "C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
  cc: { name: "C++", load: async () => (await import("@codemirror/lang-cpp")).cpp() },
  xml: { name: "XML", load: async () => (await import("@codemirror/lang-xml")).xml() },
  svg: { name: "SVG", load: async () => (await import("@codemirror/lang-xml")).xml() },
  plist: { name: "XML", load: async () => (await import("@codemirror/lang-xml")).xml() },
};

function extOf(name: string): string {
  const base = name.split(/[\\/]/).pop() ?? name;
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(dot + 1).toLowerCase() : "";
}

export function languageName(name: string): string {
  return LANGS[extOf(name)]?.name ?? "Plain text";
}

export function loadLanguage(name: string): Promise<LanguageSupport | StreamLanguage<unknown> | null> {
  const lang = LANGS[extOf(name)];
  return lang ? lang.load().catch(() => null) : Promise.resolve(null);
}
