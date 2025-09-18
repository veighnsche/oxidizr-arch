#!/usr/bin/env bash
set -euo pipefail

# Utilities to maintain AUR packaging from this directory.
# Requirements:
# - pacman-contrib (updpkgsums)
# - base-devel (makepkg)
# - namcap (optional)

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "${here}/.." && pwd)
cd "${root}"

usage() {
  cat <<EOF
Usage: $(basename "$0") <command>
Commands:
  sums           Update sha256sums in PKGBUILD using updpkgsums
  srcinfo        Generate .SRCINFO from PKGBUILD
  srcinfo-git    Generate .SRCINFO-git from PKGBUILD-git
  lint           Run namcap on PKGBUILD (and .pkg.tar.zst if present)
  build-local    Build locally with makepkg (no chroot)
EOF
}

cmd=${1:-}
case "${cmd}" in
  sums)
    updpkgsums
    ;;
  srcinfo)
    makepkg --printsrcinfo > .SRCINFO
    ;;
  srcinfo-git)
    makepkg --printsrcinfo -p PKGBUILD-git > .SRCINFO-git
    ;;
  lint)
    if command -v namcap >/dev/null 2>&1; then
      namcap -i PKGBUILD || true
      last_pkg=$(ls -1t *.pkg.tar.* 2>/dev/null | head -n1 || true)
      if [[ -n "${last_pkg}" ]]; then
        namcap -i "${last_pkg}" || true
      fi
    else
      echo "[warn] namcap not found; install 'namcap' to run lints" >&2
    fi
    ;;
  build-local)
    makepkg -si
    ;;
  *)
    usage
    exit 1
    ;;
 esac
