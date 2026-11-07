# Changelog

## 0.7.6

KlikVibe Remote: safer pairing, and a relay that doesn't give up.

- Allowing a phone shows a six-digit code, and the phone shows its own after scanning. Allow only when they match: someone who saw your QR code can no longer pass for your phone.
- The QR code's key works once: when the phone connects, it gets a new key and the one in the QR code stops working.
- A phone added again with a new key ends the connections still using the old one.
- On the Wi-Fi, Remote takes connections only from local addresses, a few at a time from each, and closes idle ones, so another device on the network can't tie it up.
- If the relay connection fails because of a bug, the Remote panel says so, the error is reported, and it tries again a minute later (before, it stopped until KlikVibe was restarted).

## 0.7.5

KlikVibe Remote: from anywhere again.

- Fixed: with Pro, a phone away from the computer's Wi-Fi (on mobile data, say) couldn't reach it ("can't reach"). The computer's link to the relay failed on its first try and never came back; it now connects and stays connected.

## 0.7.4

KlikVibe Remote: pushes say what happened.

- The push (Pro) says what happened, in the phone's language: "Waiting for an answer, or done", "Needs you" (the agent's own notification) or "Wants your attention" (the bell), with the computer's name. Still never what the agent said.

## 0.7.3

KlikVibe Remote: the whole history on the phone.

- A phone opening a terminal gets its history as the window shows it (up to 3000 lines, with colors), instead of only the last raw output, which an agent's redrawing used up fast. You can scroll back as far as on the computer.

## 0.7.2

KlikVibe Remote: close a terminal from your phone.

- Close a terminal from the phone (after asking): its tab closes on the computer, and what runs in it stops, as when closed there.
- Right-click the path above the editor (the project, a folder or the file): Reveal in Finder (Show in Explorer), show it in the file tree, copy its path or relative path.

## 0.7.1

KlikVibe Remote: start agents and send images from your phone.

- Start a new terminal from the phone in any open project, with one of your agents (Claude Code, Codex, Gemini, ...) or a plain shell. The phone can only pick an agent KlikVibe knows, never send a command of its own.
- Send an image from the phone to the computer and hand its path to the agent. Images are kept privately in a temporary folder (`klikvibe-remote`) and deleted after a week.
- Push notifications (Pro) now say which terminal needs you, so tapping one opens that terminal, and Yes/No buttons answer the agent. The push still never carries what the agent said.
- Needs KlikVibe Remote 1.0.1 on the phone; older phones keep working as before.

## 0.7.0

KlikVibe Remote: your agents, from your phone.

- Remote (the phone icon at the top right): pair the KlikVibe phone app by scanning a QR code, then watch every open project's terminals and agents from the phone, answer them, and see the changes and diffs. Each phone is allowed on the computer first, and can be removed there or in the app.
- End-to-end encrypted (Noise NNpsk0, with a key from the QR code) on the same Wi-Fi, and through the KlikVibe relay from anywhere with Pro; the open protocol is in `src-tauri/src/remote/PROTOCOL.md`.
- Agents waiting for you show on the phone too, even with desktop notifications off (and as a push notification with Pro).
- For development: `npm run tauri:dev` runs a separate "KlikVibe Dev" next to an installed KlikVibe.

## 0.6.0

Go to definition, the light way: ⌘-click or F12.

- Go to definition, without a language server: ⌘-click (Ctrl+click) or F12 on a name jumps to where it is defined (function, class, struct, fn, def, const, a method, ...), first in the file it was imported from; with several candidates a list lets you pick. In an import's quotes it opens the file (./x, ../x, $lib/x, @/x, Python modules, Rust `mod x;`). Alt+click now adds a cursor.

## 0.5.3

More ⌘-click in the terminal: folders, and file names under a folder printed above.

- More ⌘-click in the terminal: absolute folders (/tmp/x/card/, ~/Projects/app) open in Finder or Explorer, and a plain file name such as `sheet-ios-feed.png` opens when the folder it is in was printed a few lines above.

## 0.5.2

⌘-click on paths outside the project in the terminal.

- ⌘-click (Ctrl+click) also opens paths outside the project that an agent prints, such as a screenshot in a temporary folder: images, video, audio and PDFs open in their default app, anything else is shown in Finder or Explorer. Long paths that wrap onto the next line link as a whole.

## 0.5.1

The terminal takes the whole height while no file is open.

- With no file open, the terminal goes up to the top instead of sitting under the welcome card; opening a file brings the code back above it, at the terminal height you set, and closing the last tab gives the terminal the room again.

## 0.5.0

A limit for checkpoints, and how much space they take.

- Checkpoints have a limit: the last 100 are kept by default, the oldest going first, and the Checkpoints tab shows how many there are and how much space they take (e.g. "23 of 100 · 3.8 MB"). Click it to keep 50, 100, 200, 500 or all of them. Clear checkpoints has a trash icon.

## 0.4.1

A fix for KlikVibe quitting suddenly on macOS.

- Fixed: KlikVibe could quit suddenly on macOS while you worked. A harmless error inside the macOS window system ended the app instead of being logged; now it is logged and KlikVibe keeps running.

## 0.4.0

Terminals come back after a restart with the agent in its last conversation, ⌘-click on files and folders in the terminal, and tabs you can drag.

- Files and folders named in the terminal open with ⌘-click (Ctrl+click on Windows and Linux), also folders, files that .gitignore leaves out, and plain names such as `SKILL.md` when one project file goes by it. A file opens in the editor (at its line), a folder in the file tree. Web addresses open the same way; a plain click no longer opens them.
- Project tabs and file tabs can be dragged to another place in their row.
- Terminals come back when KlikVibe opens again: the same tabs and names, their last output (dimmed), and the agent that was running, started again in its last conversation (Claude Code, Codex, Gemini CLI, OpenCode, Kimi CLI, Copilot CLI and Cursor CLI; other agents are typed for you to start). Closing a project forgets its terminals.

## 0.3.2

The title bar in full screen on macOS.

- In full screen on macOS the title bar starts at the left edge, since the window buttons are hidden there.

## 0.3.1

Gitignored folders show in the file tree, file icons in the Git panel, and new installer artwork.

- The file tree lists what .gitignore leaves out (dist, node_modules, build output, store assets), dimmed, instead of hiding it. Searching by name or content still skips those files.
- Changes, commit files and checkpoint files in the Git panel show the same file-type icons as the file tree.
- The macOS disk image and the Windows installers (.exe and .msi) have KlikVibe artwork instead of the plain default look.

## 0.3.0

Stage, unstage and revert each file in the Git panel, and create, rename and delete files in the file tree.

- Staging in the Git panel: each change has buttons to revert it and to stage it (or unstage a staged one), also in its right-click menu. Staged files get a section of their own with Unstage all, the rest Stage all, and the commit button commits only what is staged ("Commit 2 staged"), or everything when nothing is.
- Files: right-click a file or folder (or the empty part of the tree, or the buttons in its header) for New file, New folder, Rename and Delete. A new name may hold folders (lib/util.ts); a new file opens; open tabs follow a rename; a checkpoint is taken before a delete, so it can be brought back.

## 0.2.0

Several projects at once, checkpoints to undo what an agent did, parallel tasks in worktrees, an MCP server for your agent, and a lot more for vibe coding.

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
- Checkpoints: as an agent works, KlikVibe saves snapshots of the files whenever they stop changing for a moment (and at least every minute), on a Git ref of their own that never touches your commits, branches, index or stash, and is never pushed. The Checkpoints tab in the Git panel lists them with their files and diffs; Restore puts the files back as they were (the current state is saved first, so a restore can be undone too).
- Ask the agent: right-click code (the selection's lines), a file, a folder, a diff, a change or a commit and pick "Ask the agent about this"; a reference such as `@src/app.ts lines 12-30` is pasted into the agent's terminal for you to finish and send. The Prompts button in the terminal holds ready-made prompts and your own.
- MCP: six more tools for your agent. `read_terminal` (the other terminals' recent output, e.g. dev server errors), `create_checkpoint` (with a note), `list_checkpoints`, `restore_checkpoint` (only after you agree in KlikVibe), `show_commit` and `reveal_in_files`. Checkpoints can carry a note, shown in the Checkpoints tab.
- KlikVibe opens again where you left it: the projects, their file tabs and the one in front, also after an update.
- Open on GitHub, GitLab or Bitbucket: the repository, the branch, a commit, or the file at the selected lines (from the Git panel, History, the editor and the breadcrumb); after pushing a feature branch, one click starts a pull (merge) request. Works with SSH and HTTPS remotes and self-hosted GitLab.
- Scripts and preview: the Scripts button in the terminal runs a package.json script (with bun, pnpm, yarn or npm, as the lockfile says) or cargo run/test/build in a new terminal. When a terminal prints a local dev server address, KlikVibe offers to open it in a preview tab (reload, address bar, open in the browser), and the terminal's tab gets a globe to open it again.
- What you haven't looked at: changes the agent made since you last opened their diff or file get a dot in the Git panel (with a count and "Mark all as seen"), the Git button in the title bar gets a dot, and checkpoints taken since you last opened the Checkpoints tab are marked New. Your own saves don't count.
- Parallel tasks: the branch button in the Git panel makes a task with its own branch and folder (a git worktree next to the repository) and opens it as a project with its own terminals, so one agent can work on it while another works in the main folder. From the task, Merge into main brings its commits back with a merge commit, and Remove deletes its folder (the branch stays). Each folder keeps its own checkpoints.

## 0.1.0

The first release of KlikVibe, a light code editor for vibe coding.

- Open a project folder (or run `klikvibe <folder>`) and reopen it from the recent list.
- A file tree that follows `.gitignore` and the disk: files changed in the last moments pulse, and changed files show their Git status.
- Code tabs with CodeMirror 6 and syntax highlighting for the common languages; open files reload when the agent edits them, and ask first when you have unsaved edits.
- A Git panel: changes with diffs, commit all, pull (fast-forward only), push or publish a branch, history with unpushed commits marked, and `git init`.
- Terminals in the project folder, as many as you need, with a button that types `claude`, `codex` or `gemini`.
- Updates: KlikVibe checks once a day and installs a new version when you agree.
- English and Bahasa Indonesia.
