#!/usr/bin/env bash
# Makes an unpacked LibreOffice self-contained (relocatable).
#
# Ubuntu's LibreOffice packages assume they live in /usr/lib/libreoffice:
# - fundamentalrc hard-codes BRAND_BASE_DIR=file:///usr/lib/libreoffice
#   (upstream LibreOffice uses a path relative to the program folder), and
# - parts of the installation are absolute links such as
#   /etc/libreoffice/registry/main.xcd, created by the package scripts.
# In a bundle (the Snap, or the development engines from
# scripts/dev-engines.sh) those paths point at the host or at nothing, and
# LibreOffice stops with a DeploymentException. This restores the relative
# base folder and rewrites every absolute link under
# <root>/usr/lib/libreoffice as a relative link into <root>.
#
# Usage: scripts/relink-libreoffice.sh <root>
set -euo pipefail
root="${1:?usage: relink-libreoffice.sh <root>}"
lo="$root/usr/lib/libreoffice"
[ -d "$lo" ] || exit 0

rc="$lo/program/fundamentalrc"
if grep -q '^BRAND_BASE_DIR=file:///usr/lib/libreoffice$' "$rc"; then
  # Replace the file rather than editing through a possible hard link.
  sed 's|^BRAND_BASE_DIR=file:///usr/lib/libreoffice$|BRAND_BASE_DIR=${ORIGIN}/..|' "$rc" > "$rc.new"
  mv "$rc.new" "$rc"
  echo "made BRAND_BASE_DIR relative in fundamentalrc"
fi
find "$lo" -type l -lname '/*' -print0 | while IFS= read -r -d '' link; do
  target="$(readlink "$link")"
  inside="$root$target"
  # The packages ship registry files in share/.registry; their install
  # scripts link them into /etc/libreoffice/registry, which a bundle lacks.
  case "$target" in
    /etc/libreoffice/registry/*)
      [ -e "$inside" ] || inside="$lo/share/.registry/$(basename "$target")"
      ;;
  esac
  if [ -e "$inside" ]; then
    ln -sfn "$(realpath --relative-to="$(dirname "$link")" "$inside")" "$link"
    echo "relinked ${link#"$root"} -> ${target}"
  else
    # Optional, writable locations (shared extension caches): not needed.
    echo "left ${link#"$root"} -> ${target} (not bundled)"
  fi
done
