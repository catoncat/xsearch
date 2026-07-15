#!/usr/bin/env bash
set -euo pipefail

REPO="${XSEARCH_REPO:-catoncat/xsearch}"
DEST="${XSEARCH_INSTALL_DIR:-$HOME/.agents/skills/xsearch}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/xsearch"

fail() {
  printf 'xsearch installer: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

platform_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os/$arch" in
    Darwin/arm64) echo "aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "x86_64-apple-darwin" ;;
    Linux/x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Linux/aarch64|Linux/arm64) echo "aarch64-unknown-linux-gnu" ;;
    *) fail "unsupported platform: $os $arch; build from source instead" ;;
  esac
}

resolve_version() {
  if [[ -n "${XSEARCH_VERSION:-}" ]]; then
    printf '%s\n' "$XSEARCH_VERSION"
    return
  fi

  local effective
  effective="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")"
  [[ "$effective" == */tag/* ]] || fail "could not resolve latest release"
  printf '%s\n' "${effective##*/}"
}

verify_checksum() {
  local file="$1" checksums="$2" expected actual
  expected="$(awk -v name="$(basename "$file")" '$2 == name { print $1 }' "$checksums")"
  [[ -n "$expected" ]] || fail "checksum missing for $(basename "$file")"

  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  else
    fail "sha256sum or shasum is required"
  fi

  [[ "$actual" == "$expected" ]] || fail "checksum verification failed"
}

main() {
  need curl
  need install
  need tar

  local target version asset base raw tmp
  target="$(platform_target)"
  version="$(resolve_version)"
  asset="xsearch-$target.tar.gz"
  base="${XSEARCH_RELEASE_BASE_URL:-https://github.com/$REPO/releases/download/$version}"
  raw="${XSEARCH_RAW_BASE_URL:-https://raw.githubusercontent.com/$REPO/$version}"
  tmp="$(mktemp -d)"
  trap "/usr/bin/find '$tmp' -depth -delete 2>/dev/null || true" EXIT

  printf 'Installing xsearch %s for %s...\n' "$version" "$target"
  curl -fsSL "$base/$asset" -o "$tmp/$asset"
  curl -fsSL "$base/checksums.txt" -o "$tmp/checksums.txt"
  verify_checksum "$tmp/$asset" "$tmp/checksums.txt"
  tar -xzf "$tmp/$asset" -C "$tmp"
  [[ -x "$tmp/xsearch" ]] || fail "release archive does not contain xsearch"

  mkdir -p "$DEST/bin" "$DEST/references" "$CONFIG_DIR"
  install -m 755 "$tmp/xsearch" "$DEST/bin/xsearch"
  curl -fsSL "$raw/SKILL.md" -o "$DEST/SKILL.md"
  curl -fsSL "$raw/config.example.toml" -o "$DEST/config.example.toml"
  curl -fsSL "$raw/references/runtime.md" -o "$DEST/references/runtime.md"
  curl -fsSL "$raw/references/leaf.md" -o "$DEST/references/leaf.md"

  if [[ ! -e "$CONFIG_DIR/config.toml" ]]; then
    install -m 600 "$DEST/config.example.toml" "$CONFIG_DIR/config.toml"
    printf 'Created %s/config.toml; add your proxy endpoint and model.\n' "$CONFIG_DIR"
  fi

  printf 'Installed binary: %s/bin/xsearch\n' "$DEST"
  printf 'Installed skill:  %s/SKILL.md\n' "$DEST"
  "$DEST/bin/xsearch" --version
}

main "$@"
