# Security

Please report security problems privately through **Report a vulnerability** on the repository's Security tab, not in a public issue. You'll get a reply within a few days.

Useful to know when looking at KlikVibe:

- **Folders:** the window can only reach folders the user opened, through the native folder dialog, the launch arguments, or the recent list, which the Rust side keeps (`src-tauri/src/projects.rs`). A path the window sends is resolved, symlinks included, and refused outside those folders. Writes inside `.git` are refused.
- **Git:** it runs the user's own `git` with arguments, never through a shell. Git is told not to prompt (`GIT_TERMINAL_PROMPT=0`), so a push that needs credentials fails instead of hanging. Commit hashes and paths from the window are validated first (`src-tauri/src/git.rs`, `commands.rs`).
- **Terminal:** shells start only in an open project folder. The app relays keystrokes and output and never runs commands on its own. *Start agent* only types the agent's name; the user presses Enter.
- **Content:** file contents and diffs are shown as text (CodeMirror and plain DOM text), never as HTML. The content security policy allows no remote scripts or connections.
- **Network:** KlikVibe sends nothing anywhere. No account, telemetry or update check.

Only the latest code on `main` gets security fixes.
