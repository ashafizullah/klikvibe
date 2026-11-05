#!/bin/sh
# Every check, on this machine: types, tests, build, licenses, Rust format,
# clippy with warnings as errors, Rust tests. Run from anywhere:
#   ./scripts/check.sh
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

echo "==> svelte-check"; bun run check
echo "==> frontend tests"; bun run test
echo "==> build"; bun run build >/dev/null
echo "==> licenses"; bun run notices
if ! git diff --quiet -- THIRD_PARTY_NOTICES.md 2>/dev/null; then
  echo "THIRD_PARTY_NOTICES.md changed: review and commit it" >&2
  exit 1
fi

cd src-tauri
echo "==> cargo fmt --check"; cargo fmt --check
echo "==> cargo clippy"; cargo clippy --all-targets -- -D warnings
echo "==> cargo test"; cargo test
if command -v cargo-deny >/dev/null 2>&1; then
  echo "==> cargo deny"; cargo deny check licenses
fi
echo "All checks passed."
