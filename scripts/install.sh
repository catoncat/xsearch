#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${XSEARCH_INSTALL_DIR:-$HOME/.agents/skills/xsearch}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/xsearch"

cargo build --release --manifest-path "$ROOT/engine/Cargo.toml"

mkdir -p "$DEST/bin" "$DEST/references" "$CONFIG_DIR"
install -m 755 "$ROOT/engine/target/release/xsearch" "$DEST/bin/xsearch"
install -m 644 "$ROOT/SKILL.md" "$DEST/SKILL.md"
install -m 644 "$ROOT/config.example.toml" "$DEST/config.example.toml"
install -m 644 "$ROOT/references/runtime.md" "$DEST/references/runtime.md"
install -m 644 "$ROOT/references/leaf.md" "$DEST/references/leaf.md"

if [[ ! -e "$CONFIG_DIR/config.toml" ]]; then
  install -m 600 "$ROOT/config.example.toml" "$CONFIG_DIR/config.toml"
  printf 'Created %s/config.toml; add your proxy endpoint and model.\n' "$CONFIG_DIR"
fi

printf 'Installed xsearch to %s\n' "$DEST"
printf 'Configure %s/config.toml or XSEARCH_* environment variables.\n' "$CONFIG_DIR"
