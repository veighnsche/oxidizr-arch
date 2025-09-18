#!/usr/bin/env bash
set -euo pipefail

# Publish dry-run for the oxidizr-arch crate
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SELF_DIR}/_common.sh"

cd_crate_root

TC="${RUST_TOOLCHAIN:-stable}"
ensure_toolchain "${TC}"

log "Packaging crate (allow-dirty to validate content)"
with_toolchain "${TC}" package -p oxidizr-arch --allow-dirty --verbose

log "cargo publish --dry-run"
with_toolchain "${TC}" publish -p oxidizr-arch --dry-run --allow-dirty --verbose

log "Dry-run publish OK"
