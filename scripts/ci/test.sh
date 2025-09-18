#!/usr/bin/env bash
set -euo pipefail

# Unit tests matrix (stable, beta, nightly) for the oxidizr-arch crate
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SELF_DIR}/_common.sh"

cd_crate_root

MATRIX_DEFAULT=(stable beta nightly)
IFS=' ' read -r -a MATRIX <<< "${RUST_MATRIX:-${MATRIX_DEFAULT[*]}}"

for TC in "${MATRIX[@]}"; do
  log "=== Running unit tests on ${TC} ==="
  ensure_toolchain "${TC}"
  log "Build (default features)"
  with_toolchain "${TC}" build -p oxidizr-arch
  log "Test (default features) — unit/integration (excluding bdd)"
  with_toolchain "${TC}" test -p oxidizr-arch -- --nocapture
  log "Test (bdd feature) — harness=false integration"
  with_toolchain "${TC}" test -p oxidizr-arch --features bdd --test bdd -q
  log "=== OK: ${TC} ==="
  echo
done

log "All matrix jobs passed"
