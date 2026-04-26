#!/usr/bin/env bash
# Download a Hugo extended binary for the given Rust target triple and
# stage it as src-tauri/binaries/hugo-<triple>(.exe) so Tauri can bundle
# it as a sidecar. Used both by `release.yml` and by anyone who wants to
# test the bundled-Hugo path locally.
#
# Usage: scripts/fetch-hugo-sidecar.sh <rust-target-triple>
# Example: scripts/fetch-hugo-sidecar.sh x86_64-apple-darwin

set -euo pipefail

HUGO_VERSION="${HUGO_VERSION:-0.139.4}"
TRIPLE="${1:-}"
if [[ -z "$TRIPLE" ]]; then
  echo "usage: $0 <rust-target-triple>" >&2
  exit 1
fi

case "$TRIPLE" in
  aarch64-apple-darwin|x86_64-apple-darwin)
    asset="hugo_extended_${HUGO_VERSION}_darwin-universal.tar.gz"
    archive_kind="tar"
    binary_in_archive="hugo"
    out_suffix=""
    ;;
  x86_64-unknown-linux-gnu)
    asset="hugo_extended_${HUGO_VERSION}_linux-amd64.tar.gz"
    archive_kind="tar"
    binary_in_archive="hugo"
    out_suffix=""
    ;;
  aarch64-unknown-linux-gnu)
    asset="hugo_extended_${HUGO_VERSION}_linux-arm64.tar.gz"
    archive_kind="tar"
    binary_in_archive="hugo"
    out_suffix=""
    ;;
  x86_64-pc-windows-msvc|i686-pc-windows-msvc)
    asset="hugo_extended_${HUGO_VERSION}_windows-amd64.zip"
    archive_kind="zip"
    binary_in_archive="hugo.exe"
    out_suffix=".exe"
    ;;
  *)
    echo "error: no Hugo extended asset known for triple '$TRIPLE'" >&2
    exit 2
    ;;
esac

dest_dir="src-tauri/binaries"
mkdir -p "$dest_dir"
out_path="$dest_dir/hugo-${TRIPLE}${out_suffix}"

if [[ -f "$out_path" ]]; then
  echo "[fetch-hugo-sidecar] $out_path already present, skipping download"
  exit 0
fi

url="https://github.com/gohugoio/hugo/releases/download/v${HUGO_VERSION}/${asset}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "[fetch-hugo-sidecar] downloading $url"
curl -fsSL "$url" -o "$tmp/$asset"

case "$archive_kind" in
  tar) tar -xzf "$tmp/$asset" -C "$tmp" "$binary_in_archive" ;;
  zip) (cd "$tmp" && unzip -o "$asset" "$binary_in_archive") ;;
esac

install -m 0755 "$tmp/$binary_in_archive" "$out_path"
echo "[fetch-hugo-sidecar] wrote $out_path"
