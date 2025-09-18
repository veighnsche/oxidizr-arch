#!/usr/bin/env bash
set -euo pipefail

# MSRV build for the oxidizr-arch crate
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SELF_DIR}/_common.sh"

cd_crate_root

MSRV_TOOLCHAIN="${MSRV_TOOLCHAIN:-1.81.0}"
ensure_toolchain "${MSRV_TOOLCHAIN}"

log "Building with MSRV ${MSRV_TOOLCHAIN} (all features)"
# Try crate-only build first; fallback to workspace build
if ! with_toolchain "${MSRV_TOOLCHAIN}" build -p oxidizr-arch --all-features; then
  with_toolchain "${MSRV_TOOLCHAIN}" build --all-features
fi

log "MSRV build OK"
