#!/usr/bin/env sh
# Shadow installer for macOS and Linux.
#   curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
# Builds from source with cargo when no release binary is available. Never touches
# system settings by itself: run `shadow install --apply` afterwards.
set -eu

REPO="${SHADOW_REPO:-https://github.com/Tiny-Bubble-Company/shadow}"
BIN_DIR="${SHADOW_BIN_DIR:-$HOME/.local/bin}"
mkdir -p "$BIN_DIR"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$arch" in arm64|aarch64) arch=aarch64 ;; x86_64|amd64) arch=x86_64 ;; esac
case "$os" in darwin) target="$arch-apple-darwin" ;; linux) target="$arch-unknown-linux-gnu" ;; *) echo "unsupported OS: $os" >&2; exit 1 ;; esac

url="$REPO/releases/latest/download/shadow-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if command -v curl >/dev/null 2>&1 && curl -fsSL "$url" -o "$tmp/shadow.tgz" 2>/dev/null; then
  tar -xzf "$tmp/shadow.tgz" -C "$tmp"
  install -m 0755 "$tmp/shadow" "$BIN_DIR/shadow"
elif command -v cargo >/dev/null 2>&1; then
  echo "No release binary for $target; building from source with cargo..."
  cargo install --locked --git "$REPO" shadow-cli --root "$tmp/root"
  install -m 0755 "$tmp/root/bin/shadow" "$BIN_DIR/shadow"
else
  echo "Need either a release for $target or a Rust toolchain (https://rustup.rs)." >&2
  exit 1
fi

# The menu-bar app on macOS: protection status and the activity dashboard. Set
# SHADOW_NO_TRAY=1 to skip it. `shadow install --apply` starts it at login when present.
if [ "$os" = darwin ] && [ -z "${SHADOW_NO_TRAY:-}" ]; then
  if [ -f "$tmp/shadow-tray" ]; then
    install -m 0755 "$tmp/shadow-tray" "$BIN_DIR/shadow-tray"
  elif command -v cargo >/dev/null 2>&1; then
    echo "Building the menu-bar app..."
    if cargo install --locked --git "$REPO" shadow-tray --root "$tmp/root"; then
      install -m 0755 "$tmp/root/bin/shadow-tray" "$BIN_DIR/shadow-tray"
    else
      echo "warning: could not build the menu-bar app; Shadow works without it." >&2
    fi
  fi
fi

"$BIN_DIR/shadow" init
echo
echo "Installed to $BIN_DIR/shadow. Next:"
echo "  shadow install          # dry run: shows exactly what will change"
echo "  shadow install --apply  # background service, menu-bar app at login, trust the local CA"
case ":$PATH:" in *":$BIN_DIR:"*) ;; *) echo "  (add $BIN_DIR to your PATH)";; esac
