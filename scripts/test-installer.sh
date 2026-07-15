#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap "/usr/bin/find '$TMP' -depth -delete 2>/dev/null || true" EXIT

case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) TARGET="aarch64-apple-darwin" ;;
  Darwin/x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux/x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Linux/aarch64|Linux/arm64) TARGET="aarch64-unknown-linux-gnu" ;;
  *) echo "unsupported test platform" >&2; exit 1 ;;
esac

mkdir -p "$TMP/release/package" "$TMP/raw/references" "$TMP/install" "$TMP/config"
cp "$ROOT/engine/target/release/xsearch" "$TMP/release/package/xsearch"
tar -C "$TMP/release/package" -czf "$TMP/release/xsearch-$TARGET.tar.gz" xsearch
(
  cd "$TMP/release"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "xsearch-$TARGET.tar.gz" > checksums.txt
  else
    shasum -a 256 "xsearch-$TARGET.tar.gz" > checksums.txt
  fi
)
cp "$ROOT/SKILL.md" "$ROOT/config.example.toml" "$TMP/raw/"
cp "$ROOT/references/runtime.md" "$ROOT/references/leaf.md" "$TMP/raw/references/"

run_installer() {
  XSEARCH_VERSION=v-test \
  XSEARCH_RELEASE_BASE_URL="file://$TMP/release" \
  XSEARCH_RAW_BASE_URL="file://$TMP/raw" \
  XSEARCH_INSTALL_DIR="$TMP/install" \
  XDG_CONFIG_HOME="$TMP/config" \
    "$ROOT/install.sh"
}

run_installer
test -x "$TMP/install/bin/xsearch"
test -f "$TMP/install/SKILL.md"
test -f "$TMP/install/references/runtime.md"
test -f "$TMP/install/references/leaf.md"
test -f "$TMP/config/xsearch/config.toml"
"$TMP/install/bin/xsearch" --version | grep -q '^xsearch '

printf '\n# preserved-user-config\n' >> "$TMP/config/xsearch/config.toml"
run_installer
grep -q 'preserved-user-config' "$TMP/config/xsearch/config.toml"

XSEARCH_INSTALL_DIR="$TMP/source-install" "$ROOT/scripts/install.sh"
test -x "$TMP/source-install/bin/xsearch"
test -f "$TMP/source-install/SKILL.md"
test -f "$TMP/source-install/references/runtime.md"
test -f "$TMP/source-install/references/leaf.md"
