#!/usr/bin/env sh
set -eu

REPO="${DOKTUI_REPO:-doklabs/doktui}"
INSTALL_DIR="${DOKTUI_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${DOKTUI_VERSION:-latest}"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
esac

case "$os" in
  linux) target="${arch}-unknown-linux-gnu" ;;
  darwin) target="${arch}-apple-darwin" ;;
  *) echo "unsupported OS: $os" >&2; exit 1 ;;
esac

api="https://api.github.com/repos/${REPO}/releases"
if [ "$VERSION" = "latest" ]; then
  url=$(curl -fsSL "$api/latest" | grep "browser_download_url.*doktui-${target}\"" | cut -d '"' -f 4 | head -n1)
else
  url=$(curl -fsSL "$api/tags/v${VERSION}" | grep "browser_download_url.*doktui-${target}\"" | cut -d '"' -f 4 | head -n1)
fi

if [ -z "$url" ]; then
  echo "no release asset found for $target" >&2
  exit 1
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

curl -fsSL "$url" -o "$tmpdir/doktui-${target}"
curl -fsSL "${url}.sha256" -o "$tmpdir/doktui-${target}.sha256" 2>/dev/null || true

if [ -f "$tmpdir/doktui-${target}.sha256" ]; then
  (cd "$tmpdir" && sha256sum -c "doktui-${target}.sha256")
fi

mkdir -p "$INSTALL_DIR"
install -m 755 "$tmpdir/doktui-${target}" "$INSTALL_DIR/doktui"
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/doktui"
echo '"script"' > "${XDG_DATA_HOME:-$HOME/.local/share}/doktui/install_method"

echo "DokTUI installed to $INSTALL_DIR/doktui"
echo "Ensure $INSTALL_DIR is in your PATH, then run: doktui"
