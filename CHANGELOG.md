# Changelog

## Unreleased

- Open several projects at once, each in its own tab in the title bar. Every project keeps its own files, Git panel and terminals, so an agent keeps running in one project while you look at another.
- Images, video, audio and PDFs open as a preview instead of "binary file", and reload when the file changes. SVGs can switch to their source, and every preview can open in its default app.
- Right-click a file tab to close it, the others, the ones to its right, the saved ones or all of them, copy its path, or reveal it in Finder or Explorer. Project tabs get the same menu for projects.

## 0.1.0

The first release of KlikVibe, a light code editor for vibe coding.

- Open a project folder (or run `klikvibe <folder>`) and reopen it from the recent list.
- A file tree that follows `.gitignore` and the disk: files changed in the last moments pulse, and changed files show their Git status.
- Code tabs with CodeMirror 6 and syntax highlighting for the common languages; open files reload when the agent edits them, and ask first when you have unsaved edits.
- A Git panel: changes with diffs, commit all, pull (fast-forward only), push or publish a branch, history with unpushed commits marked, and `git init`.
- Terminals in the project folder, as many as you need, with a button that types `claude`, `codex` or `gemini`.
- Updates: KlikVibe checks once a day and installs a new version when you agree.
- English and Bahasa Indonesia.
