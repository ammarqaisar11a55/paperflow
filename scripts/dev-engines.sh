#!/usr/bin/env bash
# Makes the processing engines available for development without root.
#
# Ghostscript and Poppler usually come with Ubuntu. qpdf and Tesseract often
# do not. This script downloads the Ubuntu packages for any that are missing
# (no installation, no sudo), unpacks them under ~/.cache/paperflow-engines
# and prints the environment variables PaperFlow reads to find them.
#
# Usage:
#   eval "$(scripts/dev-engines.sh)"
#   npm run tauri dev      # or: npm run test:rust
#
# The Snap package bundles all engines; this is for development only.
set -euo pipefail

ROOT="${PAPERFLOW_ENGINE_CACHE:-$HOME/.cache/paperflow-engines}"
DEBS="$ROOT/debs"
PREFIX="$ROOT/root"
mkdir -p "$DEBS" "$PREFIX"

need=()
command -v gs >/dev/null || need+=(ghostscript libgs10 libgs10-common)
command -v pdftoppm >/dev/null || need+=(poppler-utils libpoppler-glib8t64)
command -v qpdf >/dev/null || [ -x "$PREFIX/usr/bin/qpdf" ] || need+=(qpdf libqpdf30)
if ! command -v tesseract >/dev/null && [ ! -x "$PREFIX/usr/bin/tesseract" ]; then
  need+=(tesseract-ocr libtesseract5 libleptonica6 tesseract-ocr-eng tesseract-ocr-osd)
fi

if [ ${#need[@]} -gt 0 ]; then
  echo "Downloading: ${need[*]}" >&2
  (
    cd "$DEBS"
    for pkg in "${need[@]}"; do
      # Prefer the archive version: ESM builds need a subscription to download.
      version=$(apt-cache madison "$pkg" | awk -F'|' '$3 !~ /esm/ {gsub(/ /,"",$2); print $2; exit}')
      apt-get download "$pkg${version:+=$version}" >&2
    done
    for deb in *.deb; do dpkg -x "$deb" "$PREFIX"; done
  )
fi

echo "export PAPERFLOW_ENGINE_DIR=\"$PREFIX/usr/bin\""
echo "export PAPERFLOW_ENGINE_LIB=\"$PREFIX/usr/lib/x86_64-linux-gnu\""
