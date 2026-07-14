#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${XSEARCH_INSTALL_DIR:-$HOME/.agents/skills/xsearch}"

cargo build --release --manifest-path "$ROOT/engine/Cargo.toml"

mkdir -p "$DEST/bin"
install -m 755 "$ROOT/engine/target/release/xsearch" "$DEST/bin/xsearch"
install -m 644 "$ROOT/SKILL.md" "$DEST/SKILL.md"
install -m 644 "$ROOT/config.example.toml" "$DEST/config.example.toml"

printf 'Installed xsearch to %s\n' "$DEST"
printf 'Configure ~/.config/xsearch/config.toml or XSEARCH_* environment variables.\n'
