# Contributing to KlikVibe

Thanks for helping out! KlikVibe is maintained by one person, so a little coordination up front saves everyone time.

## Before you start

- **Bug fixes and small improvements**: open a pull request directly.
- **New features or larger changes**: open an issue first so we can agree on the approach before you write the code. KlikVibe stays small on purpose: it is a window onto an agent's work, not a full IDE.

## Development setup

Requires [Bun](https://bun.sh/), stable Rust and Git. On Linux, also install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux).

```sh
bun install
bun run tauri dev
```

## Checks

`./scripts/check.sh` runs them all; please make sure it passes before opening a pull request:

```sh
bun run check            # svelte-check
bun run test             # frontend tests
bun run build
bun run notices          # license check, regenerates THIRD_PARTY_NOTICES.md

cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check licenses
```

## Guidelines

- **Keep pull requests focused**: one fix or feature per PR.
- **UI text**: add every new string to both `src/lib/i18n/en.ts` and `src/lib/i18n/id.ts`; the type check fails if a key is missing. If you can't write Indonesian, use the English text in `id.ts` and mention it in the PR.
- **Dependencies**: ask in an issue before adding one. It must use a permissive license (MIT, Apache-2.0, BSD, ISC, Zlib; see `src-tauri/deny.toml`). GPL, AGPL and LGPL code can't be included.
- **Your own code only**: don't paste code from other projects, Stack Overflow answers or unmerged upstream pull requests. AI-assisted code is welcome, but you are responsible for understanding, testing and reviewing it before opening a PR.
- **Paths from the window are untrusted**: every new command that takes a path must go through `Projects` (`src-tauri/src/projects.rs`).
- **Commit messages**: a short summary in the imperative mood, e.g. `Reload the tree after a branch switch`.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
