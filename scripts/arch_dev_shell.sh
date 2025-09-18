#!/usr/bin/env bash
set -euo pipefail
set -x

# Interactive Arch container with oxidizr-arch built from the current repository.
# Nothing touches your host. All mutations happen inside the container.
# Usage:
#   bash scripts/arch_dev_shell.sh

IMG="archlinux:base-devel"
WORK="/work"

# Resolve repo root to the directory containing this script if possible
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SELF_DIR}/.." && pwd)"

docker run --rm -it -v "${REPO_ROOT}":"${WORK}" -w "${WORK}" "$IMG" bash -lc '
set -euo pipefail
set -x

# Update and install base tooling
pacman -Syu --noconfirm
pacman -Sy --noconfirm archlinux-keyring || true
pacman -Syu --noconfirm
pacman -S --needed --noconfirm git sudo which jq tar xz curl rust cargo base-devel

# Create a build user for AUR helper if needed
id builder >/dev/null 2>&1 || useradd -m builder
echo "%wheel ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/wheel
usermod -aG wheel builder || true

# Build oxidizr-arch from mounted workspace (works for workspace or single-crate repos)
(cargo build -p oxidizr-arch --release --locked || cargo build --release --locked)
install -Dm0755 target/release/oxidizr-arch /usr/local/bin/oxidizr-arch
oxidizr-arch --help || true

# Ensure an AUR helper (paru) is available for AUR-only packages
if ! command -v paru >/dev/null 2>&1; then
  sudo -u builder bash -lc "cd && (git clone https://aur.archlinux.org/paru-bin.git || true) && cd paru-bin && git pull --rebase || true && makepkg -si --noconfirm"
fi

export OXI_AUR_HELPER_USER=builder
cat <<EOF
You are now in a safe Arch container shell. Suggestions:
  which oxidizr-arch && oxidizr-arch --help
  oxidizr-arch doctor --json | jq .
  oxidizr-arch status --json | jq .
  # Install replacements via CLI (if implemented):
  #   oxidizr-arch --commit use coreutils
  #   oxidizr-arch --commit use findutils
  #   oxidizr-arch --commit use sudo
EOF

exec bash -l
'
