#!/usr/bin/env bash
# Regenerates THIRD-PARTY-NOTICES.md: the crates of the Windows build of
# plinth-tauri (cargo-about, configured by about.toml) and the production
# JavaScript packages of ui/.
#
#   scripts/third-party-notices.sh          rewrite THIRD-PARTY-NOTICES.md
#   scripts/third-party-notices.sh --check  exit 1 if the committed file is out of date
#
# Needs cargo-about (`cargo install cargo-about --locked`), node and pnpm.
set -euo pipefail

cd "$(dirname "$0")/.."

check=false
case "${1:-}" in
  "") ;;
  --check) check=true ;;
  *)
    echo "usage: scripts/third-party-notices.sh [--check]" >&2
    exit 2
    ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cargo fetch --locked
cargo about generate --format json -c about.toml -m crates/plinth-tauri/Cargo.toml \
  --locked --offline --fail >"$tmp/cargo-about.json"

if $check; then
  node ui/scripts/third-party-notices.mjs "$tmp/cargo-about.json" >"$tmp/THIRD-PARTY-NOTICES.md"
  if ! cmp -s "$tmp/THIRD-PARTY-NOTICES.md" THIRD-PARTY-NOTICES.md; then
    echo "THIRD-PARTY-NOTICES.md is out of date: run scripts/third-party-notices.sh" >&2
    exit 1
  fi
  echo "THIRD-PARTY-NOTICES.md is up to date"
else
  node ui/scripts/third-party-notices.mjs "$tmp/cargo-about.json" >"$tmp/THIRD-PARTY-NOTICES.md"
  mv "$tmp/THIRD-PARTY-NOTICES.md" THIRD-PARTY-NOTICES.md
fi
