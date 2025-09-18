#!/usr/bin/env bash
set -euo pipefail

# Common utilities for local CI scripts for the oxidizr-arch crate

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ -> crate root is two directories up
CRATE_ROOT="$(cd "${SELF_DIR}/../.." && pwd)"

log() {
  echo "[ci][$(date -Iseconds)] $*"
}

die() {
  echo "[ci][ERROR] $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

ensure_rustup() {
  need_cmd rustup
}

OFFLINE="${OFFLINE:-0}"

is_toolchain_installed() {
  local tc="${1:?toolchain required}"
  rustup which --toolchain "${tc}" cargo >/dev/null 2>&1
}

ensure_toolchain() {
  ensure_rustup
  local tc="${1:-stable}"
  if is_toolchain_installed "${tc}"; then
    log "Rust toolchain already installed: ${tc}"
    return 0
  fi
  if [ "${OFFLINE}" = "1" ]; then
    die "Toolchain ${tc} not installed and OFFLINE=1 set"
  fi
  log "Installing Rust toolchain: ${tc}"
  rustup toolchain install "${tc}" -q
}

ensure_components() {
  local tc="${1:?toolchain required}"; shift
  local missing=()
  local installed
  installed="$(rustup component list --installed --toolchain "${tc}" | cut -d' ' -f1 || true)"
  for comp in "$@"; do
    if ! grep -q "^${comp}$" <<<"${installed}"; then
      missing+=("${comp}")
    fi
  done
  if [ "${#missing[@]}" -eq 0 ]; then
    log "All components present for ${tc}: $*"
    return 0
  fi
  if [ "${OFFLINE}" = "1" ]; then
    die "Missing components for ${tc} in OFFLINE mode: ${missing[*]}"
  fi
  log "Adding components for ${tc}: ${missing[*]}"
  rustup component add --toolchain "${tc}" "${missing[@]}"
}

with_toolchain() {
  local tc="${1:?toolchain required}"
  shift
  RUSTUP_TOOLCHAIN="${tc}" cargo "$@"
}

cd_crate_root() {
  cd "${CRATE_ROOT}"
}
