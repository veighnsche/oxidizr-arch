#!/usr/bin/env bash
set -euo pipefail
set -x

# Non-interactive proof: disposable Arch container, build and install oxidizr-arch
# from the repository, attempt minimal diagnostics and print outputs. Host is untouched.

IMG="archlinux:base-devel"
WORKDIR="/work"

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SELF_DIR}/.." && pwd)"

docker run --rm -i -v "${REPO_ROOT}":"${WORKDIR}" -w "${WORKDIR}" "$IMG" bash -s <<'SCRIPT'
set -euo pipefail
set -x

# Ensure /etc/machine-id exists to silence noisy prompts/tools that read it
if [ ! -s /etc/machine-id ]; then
  if command -v systemd-machine-id-setup >/dev/null 2>&1; then
    systemd-machine-id-setup >/dev/null 2>&1 || true
  else
    # Fallback: generate a 32-hex machine-id from a random UUID
    tr -d '-' </proc/sys/kernel/random/uuid | head -c 32 > /etc/machine-id || true
    echo >> /etc/machine-id
  fi
fi

# Refresh packages and install prerequisites
pacman -Syu --noconfirm
pacman -S --needed --noconfirm git sudo which jq tar xz curl rust cargo base-devel python

# Create a build user for AUR helper install (optional)
id builder >/dev/null 2>&1 || useradd -m builder
echo "%wheel ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/wheel
usermod -aG wheel builder || true

# Build oxidizr-arch
(cargo build -p oxidizr-arch --release --locked || cargo build --release --locked)
OXI="target/release/oxidizr-arch"
install -Dm0755 "$OXI" /usr/local/bin/oxidizr-arch

# Pre-state diagnostics
set +e
which oxidizr-arch || true
oxidizr-arch --version || true
which ls || true
LS_PRE=$(ls --version 2>&1 | head -n2 || true)
set -e

echo "[PROOF] pre ls --version:\n${LS_PRE}"

# Ensure an AUR helper (paru) is available for AUR-only packages (optional)
if ! command -v paru >/dev/null 2>&1; then
  sudo -u builder bash -lc "cd && (git clone https://aur.archlinux.org/paru-bin.git || true) && cd paru-bin && git pull --rebase || true && makepkg -si --noconfirm" || true
fi

# CLI diagnostics
oxidizr-arch doctor --json | tee /tmp/arch_doctor.json || true
oxidizr-arch status --json | tee /tmp/arch_status.json || true

# Post-state diagnostics (tool-agnostic)
echo "[PROOF] which oxidizr-arch: $(command -v oxidizr-arch || true)"

LS_PATH="/usr/bin/ls"
TARGET=""
if [ -L "$LS_PATH" ]; then
  if command -v readlink >/dev/null 2>&1; then
    TARGET=$(readlink "$LS_PATH" || true)
  fi
fi
echo "[SH] link_target(ls): ${TARGET}"

SCRIPT
