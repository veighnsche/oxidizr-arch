# oxidizr-arch + Switchyard — Integration Guide (Arch family)

This document explains how the `oxidizr-arch` CLI composes the Switchyard library to safely migrate GNU utilities to Rust-based replacements on Arch-family distros.

- High-level consumer (CLI): `oxidizr-arch`
- Library (engine): `cargo/switchyard` (atomic swaps, backups, policy gating, logging)
- Scope of this guide: Arch family (Arch, Manjaro, CachyOS, EndeavourOS)
- Roadmap: Debian/Ubuntu and other distros will be added by additional CLIs using the same Switchyard core

---

## Why Switchyard?

Switchyard provides a safety-first, deterministic apply engine used by `oxidizr-arch` to swap targets under `/usr/bin` atomically with backups and policy gating. The core use case is moving from GNU `coreutils` to `uutils-coreutils`, but the architecture generalizes to `findutils`, `sudo-rs`, and others.

Key properties (from `cargo/switchyard/README.md`):

- Atomic, link-aware swaps with adjacent backups and rollback
- Policy gating (rescue profile, mounts, ownership, degraded EXDEV fallback)
- Deterministic IDs and redaction for DryRun/Commit parity
- Single-instance locking with bounded wait
- Optional smoke tests and attestation on success

The CLI wires distro-specific concerns (pacman/AUR, state and relink hook, MAC notes) to this stable library.

---

## Architecture

```
oxidizr-arch (CLI)
├── Experiments registry (coreutils, findutils, sudo-rs, checksums)
├── Packaging (pacman + optional AUR helper)
├── State + relink pacman hook
└── Switchyard API (plan → preflight → apply → rollback)
```

- `oxidizr-arch` resolves packages and provider binaries (e.g., `/usr/lib/uutils/coreutils`), then delegates the swap plan to Switchyard.
- Switchyard computes a `Plan`, runs `preflight` policy gating, and then performs `apply` with atomic symlink replacement and backups.

---

## Construction (default: builder)

Switchyard exposes a builder as the default way to construct the API. The CLI constructs one `Switchyard` per run and reuses it across stages.

```rust
use switchyard::api::Switchyard;
use switchyard::logging::JsonlSink;
use switchyard::policy::Policy;

fn make_api() -> Switchyard<JsonlSink, JsonlSink> {
    let facts = JsonlSink::default();
    let audit = JsonlSink::default();

    // Start from a preset and tweak per product needs
    let mut policy = Policy::production_preset();

    // EXDEV degraded fallback is typical for cross-FS symlink replacement
    // policy.apply.exdev = policy::types::ExdevPolicy::DegradedFallback;

    Switchyard::builder(facts, audit, policy)
        .with_lock_timeout_ms(30_000)
        // .with_lock_manager(Box::new(FileLockManager::new("/var/lock/switchyard.lock".into())))
        // .with_smoke_runner(Box::new(DefaultSmokeRunner::default()))
        .build()
}
```

Notes:

- The CLI owns distro lock semantics (e.g., pacman DB lock) separately from Switchyard’s process lock; both may be used.
- Emitters (`FactsEmitter`, `AuditSink`) are provided by the CLI and can target system or user log locations.

---

## CLI to Switchyard mapping

- `oxidizr-arch enable` → Switchyard `plan` → `preflight` → `apply(Commit)`
- `oxidizr-arch disable` → Switchyard `apply` with `RestoreFromBackup` actions
- `oxidizr-arch remove` → `disable` plus package uninstall (CLI) and verification
- `oxidizr-arch list-targets` → Switchyard `plan` surface only
- `oxidizr-arch check` → Switchyard `preflight` only

---

## Experiments (Arch family)

- coreutils → package `uutils-coreutils` (repo-gated). Discovery prefers a unified dispatcher if present, otherwise per-applet binaries.
- findutils → package `uutils-findutils-bin` (AUR). Requires `paru`/`yay`/`trizen`/`pamac`.
- sudo-rs → package `sudo-rs` (repo-gated). Stable aliases under `/usr/bin/*.sudo-rs`.
- checksums → presence-aware flipping for `b2sum`, `md5sum`, `sha1..sha512sum`.

Selection:

- `--experiments coreutils,findutils` or `--all` (no implicit defaults).

---

## Policy highlights used by the CLI

- Repository gating: refuse to proceed if required packages are absent in repos (or AUR helper unavailable for AUR-only packages).
- Rescue profile checks: ensure BusyBox or ≥6/10 GNU tools are present before mutations (fail-closed).
- Mounts: target mount must be `rw` and not `noexec`.
- Ownership: when `strict_ownership=true`, unknown owners stop the run unless overridden.
- Degraded EXDEV fallback: when allowed, cross-FS fallback (unlink+symlink) emits `degraded=true` telemetry.
- Smoke tests: optional; when required, post-apply verification failures emit `E_SMOKE` and may auto-rollback.

See `cargo/switchyard/README.md` for the complete policy set.

---

## State and relink hook (Arch)

- State: `/var/lib/oxidizr-arch/state.json` (override with `--state-dir`).
- Relink: `relink-managed` reads state and restores links (used by pacman hook).
- Hook: installed to `/usr/share/libalpm/hooks/oxidizr-arch-relink.hook`, runs after pacman transactions.

These features keep links consistent across package upgrades/removals.

---

## Safety model (applies via Switchyard)

- Link-aware backups adjacent to targets.
- TOCTOU-safe symlink swaps (parent FD `O_DIRECTORY|O_NOFOLLOW`, then `renameat`, `fsync(parent)`).
- Idempotent apply and restore semantics.
- Deterministic IDs and redaction in DryRun for stable facts.
- Single-instance process lock; CLI also respects pacman’s DB lock.

---

## Examples

```bash
# Dry-run enabling all experiments and wait up to 30s for pacman lock
oxidizr-arch --all --dry-run --wait-lock 30 enable

# Enable only coreutils, skip pacman -Sy, select paru as AUR helper
sudo oxidizr-arch --experiments coreutils --no-update --aur-helper paru enable

# Flip checksum tools explicitly (presence-aware)
sudo oxidizr-arch --experiments checksums enable

# Disable only coreutils (restore-only)
sudo oxidizr-arch --experiments coreutils disable

# Remove coreutils (restore originals, then uninstall package)
sudo oxidizr-arch --experiments coreutils remove
```

---

## Debian/Ubuntu and beyond (roadmap)

The CLI is purpose-built for Arch today. For Debian/Ubuntu and other distros, the plan is to create distro-specific CLIs that:

- Reuse the same Switchyard library and policies
- Replace package resolution (APT/dpkg) and post-transaction hooks
- Keep the apply engine, safety model, and logging identical

This keeps the core migration (GNU → uutils) consistent across distros.

---

## References

- Library guide: `cargo/switchyard/README.md`
- CLI README (Arch): `README.md`
- Orchestrated testing: `test-orch/`
- Policy: `cargo/switchyard/src/policy/`
- Logging facade: `cargo/switchyard/src/logging/`
