#!/usr/bin/env bash
set -euo pipefail

# Smoke test inside Arch container: build oxidizr-arch and run status/doctor
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SELF_DIR}/_common.sh"

cd_crate_root

need_cmd docker

ART_DIR="${CRATE_ROOT}/.artifacts/smoke"
mkdir -p "${ART_DIR}"

IMG="archlinux:base-devel"
WORK="/work"

log "Running smoke in Docker image: ${IMG}"

# Use --pull=missing to avoid unnecessary network if image already present
# Note: The container performs networked package installs; this mirrors CI.
docker run --pull=missing --rm -v "${CRATE_ROOT}":"${WORK}" -w "${WORK}" "$IMG" bash -lc '
set -euo pipefail
set -x

# Update and install base tooling
pacman -Syu --noconfirm
pacman -S --needed --noconfirm git sudo which jq tar xz curl rust cargo base-devel python

# Build binary (works for workspace or single-crate)
(cargo build -p oxidizr-arch --release || cargo build --release)
install -Dm0755 target/release/oxidizr-arch /usr/local/bin/oxidizr-arch

ROOT=$(mktemp -d)
mkdir -p "$ROOT/usr/bin" "$ROOT/var/lock"

set +e
oxidizr-arch --root "$ROOT" status --json | tee "/work/.artifacts/smoke/arch_status.json"
oxidizr-arch --root "$ROOT" doctor --json | tee "/work/.artifacts/smoke/arch_doctor.json"
set -e
'

log "Smoke artifacts written to: ${ART_DIR}"
