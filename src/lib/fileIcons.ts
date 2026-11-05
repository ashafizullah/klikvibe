// An icon and colour per kind of file, for the file tree, search results and
// editor tabs. Icon names are Icon.svelte's; the colours are picked to stay
// readable on the dark panels (3:1 or more, checked in fileIcons.test.ts).

export interface FileIcon {
  icon: string;
  color: string;
}

const C = {
  ts: "#4f9cf0",
  js: "#e8c547",
  svelte: "#ff6a3d",
  vue: "#41b883",
  html: "#e8673b",
  css: "#7b95f0",
  sass: "#d06a9c",
  rust: "#dea584",
  go: "#4fc3d9",
  python: "#5ea3e0",
  ruby: "#e0564d",
  php: "#8f93c9",
  java: "#e0803a",
  kotlin: "#a97bff",
  swift: "#f05138",
  c: "#7f9cd1",
  csharp: "#a57fe0",
  dart: "#3db5e8",
  shell: "#8fbf6a",
  json: "#e8c547",
  markdown: "#8fa6c4",
  config: "#a89f8c",
  data: "#c3a3e3",
  image: "#45b884",
  video: "#d07bd9",
  audio: "#e58b5a",
  pdf: "#ec5f63",
  archive: "#c49a6c",
  key: "#e3b341",
  font: "#c99be6",
  git: "#f05033",
  docker: "#3ba3f0",
  npm: "#e05a52",
  license: "#d4b03a",
  readme: "#6ea8fe",
  muted: "#9a917f",
} as const;

const code = (color: string): FileIcon => ({ icon: "code", color });

/** By extension (lower case). */
const BY_EXT: Record<string, FileIcon> = {
  ts: code(C.ts), tsx: code(C.ts), mts: code(C.ts), cts: code(C.ts), "d.ts": code(C.ts),
  js: code(C.js), jsx: code(C.js), mjs: code(C.js), cjs: code(C.js),
  svelte: code(C.svelte), vue: code(C.vue), astro: code(C.svelte),
  html: code(C.html), htm: code(C.html), xml: code(C.html), svg: { icon: "image", color: C.image },
  css: code(C.css), scss: code(C.sass), sass: code(C.sass), less: code(C.css),
  rs: code(C.rust), go: code(C.go), py: code(C.python), rb: code(C.ruby), php: code(C.php),
  java: code(C.java), kt: code(C.kotlin), kts: code(C.kotlin), swift: code(C.swift),
  c: code(C.c), h: code(C.c), cpp: code(C.c), cc: code(C.c), hpp: code(C.c), cs: code(C.csharp), dart: code(C.dart),
  sh: { icon: "file-terminal", color: C.shell }, bash: { icon: "file-terminal", color: C.shell }, zsh: { icon: "file-terminal", color: C.shell },
  fish: { icon: "file-terminal", color: C.shell }, ps1: { icon: "file-terminal", color: C.shell }, bat: { icon: "file-terminal", color: C.shell }, cmd: { icon: "file-terminal", color: C.shell },
  json: { icon: "braces", color: C.json }, jsonc: { icon: "braces", color: C.json }, json5: { icon: "braces", color: C.json },
  md: { icon: "file-text", color: C.markdown }, mdx: { icon: "file-text", color: C.markdown }, txt: { icon: "file-text", color: C.muted }, rst: { icon: "file-text", color: C.markdown },
  yaml: { icon: "file-cog", color: C.config }, yml: { icon: "file-cog", color: C.config }, toml: { icon: "file-cog", color: C.config },
  ini: { icon: "file-cog", color: C.config }, conf: { icon: "file-cog", color: C.config }, cfg: { icon: "file-cog", color: C.config },
  sql: { icon: "database", color: C.data }, db: { icon: "database", color: C.data }, sqlite: { icon: "database", color: C.data },
  csv: { icon: "file-spreadsheet", color: C.image }, tsv: { icon: "file-spreadsheet", color: C.image }, xlsx: { icon: "file-spreadsheet", color: C.image }, xls: { icon: "file-spreadsheet", color: C.image },
  png: { icon: "image", color: C.image }, jpg: { icon: "image", color: C.image }, jpeg: { icon: "image", color: C.image }, gif: { icon: "image", color: C.image },
  webp: { icon: "image", color: C.image }, avif: { icon: "image", color: C.image }, bmp: { icon: "image", color: C.image }, ico: { icon: "image", color: C.image }, icns: { icon: "image", color: C.image },
  mp4: { icon: "video", color: C.video }, m4v: { icon: "video", color: C.video }, webm: { icon: "video", color: C.video }, mov: { icon: "video", color: C.video }, ogv: { icon: "video", color: C.video },
  mp3: { icon: "audio", color: C.audio }, wav: { icon: "audio", color: C.audio }, ogg: { icon: "audio", color: C.audio }, oga: { icon: "audio", color: C.audio },
  m4a: { icon: "audio", color: C.audio }, aac: { icon: "audio", color: C.audio }, flac: { icon: "audio", color: C.audio },
  pdf: { icon: "pdf", color: C.pdf },
  zip: { icon: "file-archive", color: C.archive }, tar: { icon: "file-archive", color: C.archive }, gz: { icon: "file-archive", color: C.archive },
  tgz: { icon: "file-archive", color: C.archive }, rar: { icon: "file-archive", color: C.archive }, "7z": { icon: "file-archive", color: C.archive },
  dmg: { icon: "file-archive", color: C.archive }, xz: { icon: "file-archive", color: C.archive },
  ttf: { icon: "file-type", color: C.font }, otf: { icon: "file-type", color: C.font }, woff: { icon: "file-type", color: C.font }, woff2: { icon: "file-type", color: C.font },
  pem: { icon: "file-key", color: C.key }, key: { icon: "file-key", color: C.key }, crt: { icon: "file-key", color: C.key }, p12: { icon: "file-key", color: C.key },
  lock: { icon: "file-lock", color: C.muted }, lockb: { icon: "file-lock", color: C.muted },
};

/** Whole file names (lower case) that mean more than their extension. */
const BY_NAME: Record<string, FileIcon> = {
  "package.json": { icon: "package", color: C.npm },
  "cargo.toml": { icon: "package", color: C.rust },
  "go.mod": { icon: "package", color: C.go },
  "pyproject.toml": { icon: "package", color: C.python },
  "composer.json": { icon: "package", color: C.php },
  gemfile: { icon: "package", color: C.ruby },
  "package-lock.json": { icon: "file-lock", color: C.muted },
  "pnpm-lock.yaml": { icon: "file-lock", color: C.muted },
  "yarn.lock": { icon: "file-lock", color: C.muted },
  "go.sum": { icon: "file-lock", color: C.muted },
  dockerfile: { icon: "container", color: C.docker },
  ".dockerignore": { icon: "container", color: C.docker },
  "docker-compose.yml": { icon: "container", color: C.docker },
  "docker-compose.yaml": { icon: "container", color: C.docker },
  "compose.yml": { icon: "container", color: C.docker },
  "compose.yaml": { icon: "container", color: C.docker },
  ".gitignore": { icon: "branch", color: C.git },
  ".gitattributes": { icon: "branch", color: C.git },
  ".gitmodules": { icon: "branch", color: C.git },
  ".editorconfig": { icon: "file-cog", color: C.config },
  makefile: { icon: "file-terminal", color: C.shell },
  justfile: { icon: "file-terminal", color: C.shell },
};

/** The icon and colour for a file name. */
export function fileIcon(name: string): FileIcon {
  const lower = name.toLowerCase();
  if (BY_NAME[lower]) return BY_NAME[lower];
  if (/^readme(\.|$)/.test(lower)) return { icon: "book", color: C.readme };
  if (/^(license|licence|copying)(\.|$)/.test(lower)) return { icon: "scale", color: C.license };
  if (/^\.env(\.|$)/.test(lower) || lower.endsWith(".env")) return { icon: "file-key", color: C.key };
  if (/^dockerfile\./.test(lower)) return { icon: "container", color: C.docker };
  // Tool settings: tsconfig.json, vite.config.ts, .prettierrc, .eslintrc.cjs …
  if (/^(tsconfig|jsconfig)(\..+)?\.json$/.test(lower) || /\.config\.[cm]?[jt]s$/.test(lower) || /^\.[\w-]+rc(\.\w+)?$/.test(lower)) {
    return { icon: "file-cog", color: C.config };
  }
  if (lower.endsWith(".d.ts")) return BY_EXT["d.ts"];
  const dot = lower.lastIndexOf(".");
  return (dot > 0 && BY_EXT[lower.slice(dot + 1)]) || { icon: "file", color: C.muted };
}

/** Every colour in use, for the contrast test. */
export const FILE_ICON_COLORS = Object.values(C);
