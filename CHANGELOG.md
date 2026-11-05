# Changelog

## Unreleased

- Open several projects at once, each in its own tab in the title bar. Every project keeps its own files, Git panel and terminals, so an agent keeps running in one project while you look at another.
- Images, video, audio and PDFs open as a preview instead of "binary file", and reload when the file changes. SVGs can switch to their source, and every preview can open in its default app.
- Right-click a file tab to close it, the others, the ones to its right, the saved ones or all of them, copy its path, or reveal it in Finder or Explorer. Project tabs get the same menu for projects.
- Right-click a change in the Git panel to open it, discard it (or delete a new file), add it or its folder to .gitignore, copy its path or reveal it. Right-click a commit to copy its hash or message.
- Right-click files and folders in the file tree, and projects in the recent list, for the same kind of menu.
- A local MCP server (off by default) for the agent you use: it can read the file and selection you are looking at and your unsaved edits, and open files and diffs in KlikVibe. Setup steps for Claude Code, Codex, Gemini CLI, OpenCode, Oh My Pi, Command Code, Amp, Qwen Code, Kimi CLI, Droid, Copilot CLI, Crush and Cursor CLI.
- *Start agent* knows more agents: OpenCode, Oh My Pi (`omp`), Command Code (`cmd`), Amp, Qwen Code, Kimi CLI, Droid, Copilot CLI, Crush, Cursor CLI and Aider.
- A hidden terminal leaves a strip at the bottom of the editor to bring it back, and the title bar shows a terminal icon for it.
- Updates the way KlikPaper does them: install automatically (download in the background, install on quit), ask first, or only check on request. A new version shows its changes in your language, and after an update KlikVibe shows once what's new.
- An *About & updates* window (the ⓘ button in the title bar, or the version on Home) with the version, the update choice and a Check for updates button.
- A side-by-side layout: Terminal | Code | Git as columns (the button next to the panel toggles). Every panel can be hidden, the code included (`⌘⇧E` / `Ctrl+Shift+E`), so the terminal can take the whole window; at least one stays on screen. The file tree starts hidden in this layout (`⌘B` shows it), and each layout remembers its own choice. Terminals keep running when the layout changes.
- The welcome card stays behind the terminal when the terminal is dragged up.
- Tooltips on every button: hover an icon to see what it does and its shortcut.
- A commit graph in the Git history: branch and merge lines, branch and tag labels, the checked-out commit marked, and a switch between this branch and all branches. Click a commit to list the files it changed (with lines added and removed) and open each one's diff; the whole commit is one click further.
- Diffs side by side: the old version on the left, the new on the right, an edited line's two versions on one row with the changed part highlighted. A switch in the diff header goes back to one column.
- Questions such as "Discard changes?" use KlikVibe's own dialog instead of the system one; for anything that can't be undone the button is red and Cancel has the focus.
- The faint film-grain texture over the window is gone: it was barely visible and cost about 40 MB of graphics memory while KlikVibe is in front.
- Right-click in the code editor for Cut, Copy, Paste, Select all, Find and Go to line, plus Save, the file's changes, its path and Reveal; in a diff for Copy, Select all, the layout switch and the file.
- Search in the Files panel: by name (`⌘P`), with words, `*`, `**` and `?` patterns and `!` to leave out, or in file contents (`⌘⇧F`), optionally only in files like `*.ts`; a result opens the file at that line. Both follow `.gitignore`.
- Shortcuts no longer fire on Ctrl in the terminal: on macOS they use ⌘ only, elsewhere Ctrl+letter goes to the shell while the terminal has the keyboard (Ctrl+W no longer closes a file tab).
- File icons follow the kind of file: code by language colour, JSON, config, lockfiles, `.env`, images, video, audio, archives, `package.json`, `Cargo.toml`, `Dockerfile`, README and LICENSE each have their own, in the tree, search results and tabs.
- KlikVibe tells you when an agent needs you: after it has been working and goes quiet (waiting for an answer, or done), or rings the terminal bell or sends a notification. Unless you are looking at that terminal, its project tab gets a bell, the Dock icon bounces and macOS shows a notification. The bell button in the terminal header turns it off.
- Paths in terminal output (`src/app.ts:42:7`, `--> src/main.rs:10:5`, `app.ts(12,5)`) open the file at that line with a click, when the file is in the project; web addresses such as `http://localhost:5173` open in the browser.
- Terminal tabs are named after what runs in them ("Claude Code", "npm run dev", "vite"), and back to "Terminal 1" at the shell prompt. Double-click or right-click a tab to rename it; your name stays until you pick the automatic one again.
- A breadcrumb over the editor (project › folders › file): a folder lists its files to open one, the file offers Show in Files, its path and Reveal. Show in Files opens the folders down to it in the tree.

## 0.1.0

The first release of KlikVibe, a light code editor for vibe coding.

- Open a project folder (or run `klikvibe <folder>`) and reopen it from the recent list.
- A file tree that follows `.gitignore` and the disk: files changed in the last moments pulse, and changed files show their Git status.
- Code tabs with CodeMirror 6 and syntax highlighting for the common languages; open files reload when the agent edits them, and ask first when you have unsaved edits.
- A Git panel: changes with diffs, commit all, pull (fast-forward only), push or publish a branch, history with unpushed commits marked, and `git init`.
- Terminals in the project folder, as many as you need, with a button that types `claude`, `codex` or `gemini`.
- Updates: KlikVibe checks once a day and installs a new version when you agree.
- English and Bahasa Indonesia.
