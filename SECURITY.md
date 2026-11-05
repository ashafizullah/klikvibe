# Security

Please report security problems privately through **Report a vulnerability** on the repository's Security tab, not in a public issue. You'll get a reply within a few days.

Useful to know when looking at KlikVibe:

- **Folders:** the window can only reach folders the user opened, through the native folder dialog, the launch arguments, or the recent list, which the Rust side keeps (`src-tauri/src/projects.rs`). A path the window sends is resolved, symlinks included, and refused outside those folders. Writes inside `.git` are refused.
- **Git:** it runs the user's own `git` with arguments, never through a shell. Git is told not to prompt (`GIT_TERMINAL_PROMPT=0`), so a push that needs credentials fails instead of hanging. Commit hashes and paths from the window are validated first (`src-tauri/src/git.rs`, `commands.rs`).
- **Terminal:** shells start only in an open project folder. The app relays keystrokes and output and never runs commands on its own. *Start agent* only types the agent's name; the user presses Enter.
- **Media:** images, video, audio and PDFs load over a `media://` protocol (`src-tauri/src/media.rs`) that serves only those file types, and only from open project folders. *Open in default app* is limited to the same types, so it can't run a script or program.
- **MCP server (off by default):** when the user turns it on, it listens on `127.0.0.1` only, needs a random secret token (kept in the app data folder), and refuses requests from web pages on other origins (`src-tauri/src/mcp.rs`). Its tools read what is open in the editor and open files and diffs inside open projects; none of them types in the terminal, writes or deletes files, or runs Git.
- **Checkpoints:** snapshots are commits on `refs/klikvibe/checkpoints` built from a separate index (`.git/klikvibe-index`); they never touch the user's index, HEAD, branches or stash, and `git push` leaves the ref alone. Restoring rewrites files changed since the checkpoint and deletes files added since, inside the project only, after saving the current state as a checkpoint.
- **Content:** file contents and diffs are shown as text (CodeMirror and plain DOM text), never as HTML. The content security policy allows no remote scripts or connections.
- **Network:** no account or telemetry. KlikVibe checks GitHub for updates once a day (unless the user turns that off), fetches a new version's changelog from the repository, and installs only packages signed with the project's key. The MCP server (when on) only accepts local connections.

Only the latest code on `main` gets security fixes.
