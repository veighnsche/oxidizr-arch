#!/usr/bin/env bash
set -euo pipefail

# Lint & format for the oxidizr-arch crate
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SELF_DIR}/_common.sh"

cd_crate_root

TC="${RUST_TOOLCHAIN:-stable}"
ensure_toolchain "${TC}"
log "Ensuring components on ${TC}: rustfmt, clippy"
ensure_components "${TC}" rustfmt clippy

log "Format check"
# Prefer crate-only format, fallback to workspace-wide if unsupported
if ! with_toolchain "${TC}" fmt -p oxidizr-arch -- --check; then
  with_toolchain "${TC}" fmt -- --check
fi

log "Clippy (warnings as errors)"
with_toolchain "${TC}" clippy -p oxidizr-arch --all-targets -- -D warnings

# Zero-SKIP gate
if [ -d tests ]; then
  log "Zero-SKIP gate: scanning tests/ for #[ignore]"
  if grep -R --line-number -E '^[[:space:]]*#\[ignore\]' tests; then
    die "#[ignore] present in tests; Zero-SKIP gate requires no skipped tests."
  fi
fi

# Changelog gate (only when a parent commit exists)
if git rev-parse --verify HEAD^ >/dev/null 2>&1; then
  log "Changelog gate: require CHANGELOG.md for source changes"
  changed=$(git diff --name-only HEAD^)
  if echo "$changed" | grep -E '^(src/|Cargo.toml|SPEC/|docs/|hack/|tests/)'; then
    echo "$changed" | sed 's/^/ - /'
    if ! echo "$changed" | grep -q '^CHANGELOG.md$'; then
      die "Detected crate changes without updating CHANGELOG.md"
    fi
  fi
else
  log "No parent commit; skipping changelog gate"
fi

log "Lint OK"
