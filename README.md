<p align="center">
  <img src="design/header.png" alt="KlikVibe: let the agent code, keep an eye on it">
</p>

<h1 align="center">KlikVibe</h1>

<p align="center"><b>English</b> · <a href="README.id.md">Bahasa Indonesia</a></p>

<p align="center">
  A free, open-source, lightweight code editor for vibe coding. Let your AI agent write the code in the built-in terminal, and keep an eye on it: files, diffs, Git history and pushes in one small, fast window.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white" alt="Svelte 5">
  <img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust" alt="Rust">
  <a href="https://github.com/ashafizullah/klikvibe/releases/latest"><img src="https://img.shields.io/github/v/release/ashafizullah/klikvibe?include_prereleases&label=release" alt="Release"></a>
  <a href="https://github.com/ashafizullah/klikvibe/releases"><img src="https://img.shields.io/github/downloads/ashafizullah/klikvibe/total?color=7c3aed" alt="Downloads"></a>
</p>

<p align="center">
  Part of <a href="https://klikapps.pages.dev"><b>Klik Apps</b></a>, a family of free, open-source desktop apps: <a href="https://klikapps.pages.dev">klikapps.pages.dev</a>
</p>

## What it does

- **Terminal first.** Shells open in the project folder, as many as you need (the agent in one, the dev server in another). *Start agent* types the command of your agent (Claude Code, Codex, Gemini CLI, OpenCode, Oh My Pi, Command Code, Amp, Qwen Code, Kimi CLI, Droid, Copilot CLI, Crush, Cursor CLI or Aider) for you; you press Enter.
- **Several projects at once,** each in its own tab with its own files, Git panel and terminals, so an agent keeps working in one while you look at another.
- **Your agent sees what you see (MCP).** Turn on the local MCP server and add it to your agent with one command: it can then read the file and the code you have selected, see unsaved edits and your other terminals' output (dev server errors, failing tests), open files, diffs and commits in KlikVibe for you to review, and take checkpoints before risky changes.
- **Knows when the agent needs you.** When an agent has been working and goes quiet (waiting for an answer, or done), KlikVibe shows a notification, so you can do something else meanwhile. Paths in its output open the file at that line with a click.
- **Undo what the agent did.** Checkpoints save your files as the agent works, apart from your commits; restore one to go back to before a change.
- **Point the agent at code.** Right-click code, a file or a diff and pick "Ask the agent about this" to paste a reference into its terminal.
- **Live view of the agent's work.** The file tree, open files and the Git panel follow the disk as files change. Files touched in the last moments pulse in the tree, and open files reload by themselves unless you have unsaved edits there.
- **Code when you want it.** CodeMirror 6 with syntax highlighting for the common languages, search, and editing with save (`⌘S` / `Ctrl+S`). Images, video, audio and PDFs open as a preview; other binary files and very large files are not opened.
- **Git without leaving.**
  - Changes, each with a diff.
  - Commit all with a message (`⌘↵`).
  - Pull (fast-forward only), push, or publish a new branch.
  - History, with unpushed commits marked; click a commit to see its patch.
  - `git init` for a new folder.

  It runs your own `git`, so your credentials, SSH keys and hooks work as in a terminal.
- **Small and offline.** Tauri 2 + Svelte 5 + Rust. No account, no telemetry, no cloud: the optional MCP server only listens on your own computer.
- English and Bahasa Indonesia.

## Install

**macOS** (Apple Silicon + Intel, macOS 11 or later): run this in Terminal to install or update:
```sh
curl -fsSL https://raw.githubusercontent.com/ashafizullah/klikvibe/main/install.sh | bash
```
Or download `KlikVibe_*_universal.dmg` from the [latest release](https://github.com/ashafizullah/klikvibe/releases/latest), open it and drag KlikVibe into Applications. It is signed and notarized by Apple.

**Windows**: download `KlikVibe_*_x64-setup.exe` (or the `.msi`) from the [latest release](https://github.com/ashafizullah/klikvibe/releases/latest).

**Linux** (x86_64): download the `.deb`, `.rpm` or `.AppImage`.

KlikVibe checks for a new version once a day and installs it when you agree. Git must be installed; the agent you want to run (Claude Code, Codex, Gemini CLI) too.

## Build from source

You need [Bun](https://bun.sh/), stable Rust and Git. On Linux, also install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux).

```sh
bun install
bun run tauri dev                    # run the app
bun run tauri dev -- -- ~/my-app     # run it with a project folder open
bun run tauri build                  # bundle
```

A running KlikVibe also opens a folder passed to a second launch (`klikvibe <folder>`).

## Shortcuts

| | macOS | Windows / Linux |
|---|---|---|
| Open folder | ⌘O | Ctrl+O |
| Open another project (new project tab) | ⌘T | Ctrl+T |
| Next / previous project | Ctrl+Tab / Ctrl+Shift+Tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Go to project 1–9 | ⌘1–⌘9 | Ctrl+1–Ctrl+9 |
| Save | ⌘S | Ctrl+S |
| Close tab | ⌘W | Ctrl+W |
| Files panel | ⌘B | Ctrl+B |
| Code panel | ⌘⇧E | Ctrl+Shift+E |
| Find a file by name | ⌘P | Ctrl+P |
| Search in files | ⌘⇧F | Ctrl+Shift+F |
| Git panel | ⌘⇧G | Ctrl+Shift+G |
| Terminal | Ctrl+` | Ctrl+` |
| Commit (in the message box) | ⌘↵ | Ctrl+Enter |

## Security

The window can only reach folders you opened yourself, through the folder dialog, the launch arguments, or the recent list kept by the Rust side. Every file and Git command checks its path against them, with symlinks resolved, and KlikVibe never writes inside `.git`. See [SECURITY.md](SECURITY.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). `./scripts/check.sh` runs every check.

## License

[MIT](LICENSE)

KlikVibe is built on:

- [Tauri](https://tauri.app) (MIT OR Apache-2.0)
- [CodeMirror](https://codemirror.net) (MIT)
- [xterm.js](https://xtermjs.org) (MIT)
- [portable-pty](https://github.com/wezterm/wezterm) (MIT)
- [notify](https://github.com/notify-rs/notify) (CC0-1.0)
- [ignore](https://github.com/BurntSushi/ripgrep) (MIT OR Unlicense)

It also bundles the Lucide icons (ISC) and the interface fonts Fraunces, IBM Plex Sans and IBM Plex Mono (SIL Open Font License 1.1). Every third-party license is in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md). Run `bun run notices` after changing dependencies; it fails on a license outside the permissive allow-list.

Claude and Claude Code are trademarks of Anthropic. Codex is a trademark of OpenAI. Gemini is a trademark of Google LLC. Git is a trademark of the Software Freedom Conservancy. Other product names are trademarks of their owners. KlikVibe is an independent project and is not affiliated with or endorsed by any of them.

<p>
  <a href="https://github.com/ashafizullah"><img src="https://github.com/ashafizullah.png?size=64" width="32" height="32" alt="" align="left" style="border-radius:50%"></a>
  &nbsp;Built by <a href="https://github.com/ashafizullah"><b>Adam Suchi Hafizullah</b></a>
</p>
