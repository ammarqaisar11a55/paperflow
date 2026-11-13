#!/usr/bin/env bash
# Makes the processing engines available for development without root.
#
# Ghostscript and Poppler usually come with Ubuntu. qpdf, Tesseract and
# LibreOffice often do not. This script downloads the Ubuntu packages for any
# that are missing (no installation, no sudo), unpacks them under
# ~/.cache/paperflow-engines and prints the environment variables PaperFlow
# reads to find them. LibreOffice and its dependencies are a few hundred MB.
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

if ! command -v soffice >/dev/null && [ ! -x "$PREFIX/usr/lib/libreoffice/program/soffice" ]; then
  # Every dependency the host lacks, resolved by apt.
  lo=$(apt-cache depends --recurse --no-recommends --no-suggests --no-conflicts --no-breaks \
    --no-replaces --no-enhances libreoffice-writer-nogui libreoffice-calc-nogui \
    libreoffice-impress-nogui fonts-crosextra-carlito fonts-crosextra-caladea fonts-liberation \
    2>/dev/null | grep -E '^[a-z0-9]' | sort -u)
  for pkg in $lo; do dpkg -s "$pkg" >/dev/null 2>&1 || need+=("$pkg"); done
fi

if [ ${#need[@]} -gt 0 ]; then
  echo "Downloading: ${need[*]}" >&2
  (
    cd "$DEBS"
    for pkg in "${need[@]}"; do
      # Prefer the archive version: ESM builds need a subscription to download.
      version=$(apt-cache madison "$pkg" | awk -F'|' '$3 !~ /esm/ {gsub(/ /,"",$2); print $2; exit}')
      apt-get download "$pkg${version:+=$version}" >&2 || echo "skipped $pkg" >&2
    done
    for deb in *.deb; do dpkg -x "$deb" "$PREFIX"; done
  )
  bash "$(dirname "$0")/relink-libreoffice.sh" "$PREFIX" >&2
fi

echo "export PAPERFLOW_ENGINE_DIR=\"$PREFIX/usr/bin\""
echo "export PAPERFLOW_ENGINE_LIB=\"$PREFIX/usr/lib/x86_64-linux-gnu\""
