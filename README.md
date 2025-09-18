# oxidizr-arch — Arch Linux CLI to use Rust replacements safely

[![Lint](https://github.com/veighnsche/oxidizr-arch/actions/workflows/lint.yml/badge.svg)](https://github.com/veighnsche/oxidizr-arch/actions/workflows/lint.yml)
[![Test](https://github.com/veighnsche/oxidizr-arch/actions/workflows/test.yml/badge.svg)](https://github.com/veighnsche/oxidizr-arch/actions/workflows/test.yml)
[![MSRV](https://github.com/veighnsche/oxidizr-arch/actions/workflows/msrv.yml/badge.svg)](https://github.com/veighnsche/oxidizr-arch/actions/workflows/msrv.yml)
[![Smoke](https://github.com/veighnsche/oxidizr-arch/actions/workflows/smoke.yml/badge.svg)](https://github.com/veighnsche/oxidizr-arch/actions/workflows/smoke.yml)

oxidizr-arch is a small, safety-first CLI that switches key system toolchains to their
Rust replacements on Arch and derivatives (GNU coreutils → uutils-coreutils, findutils → uutils-findutils, sudo → sudo-rs).
It performs safe, atomic, reversible changes under the hood via the Switchyard engine and keeps a one-step restore path.

This CLI focuses on safety and UX:

- You do not choose applets, sources, or targets manually.
- The CLI ensures the right replacement is installed (pacman/AUR) and applies it safely.
- You can restore to GNU/stock tools at any time.

## Powered by Switchyard-fs

[![Switchyard CI](https://github.com/veighnsche/switchyard/actions/workflows/test.yml/badge.svg)](https://github.com/veighnsche/switchyard/actions/workflows/test.yml)
[![Crates.io](https://img.shields.io/crates/v/switchyard-fs.svg)](https://crates.io/crates/switchyard-fs)
[![docs.rs](https://img.shields.io/docsrs/switchyard-fs)](https://docs.rs/switchyard-fs)
[![mdBook](https://img.shields.io/badge/book-mdBook-blue)](https://veighnsche.github.io/switchyard/)

oxidizr-arch is powered by the Switchyard engine, which provides the safe, deterministic apply/rollback core used by this CLI. Switchyard handles atomic symlink swaps with backup/restore, preflight policy gates, rescue verification, locking, optional smoke checks with auto‑rollback, and structured facts/audit emission.

- Project README: [cargo/switchyard/README.md](../switchyard/README.md)
- Operator & Integrator Guide (mdBook): <https://veighnsche.github.io/switchyard/>
- API docs on docs.rs: <https://docs.rs/switchyard-fs>

## Responsibilities

- CLI (oxidizr-arch)
  - UX and ergonomics: simple commands (`status`, `doctor`, `use`, `restore`, `replace`) with friendly output and `--json` variants.
  - Arch‑specific orchestration: ensure/verify packages via pacman/AUR; select correct replacements and paths; detect pacman locks; provide containerized CI smoke runs.
  - Planning and wiring: choose a Switchyard policy preset suitable for Arch switches; construct source→target symlink plans and invoke Switchyard; provide adapters (e.g., file lock manager) as needed.
  - Distro guardrails: refuse to mutate when prerequisites are missing; keep a one‑step restore path; never change the host unless `--commit` is provided.

- Switchyard
  - Core engine: `SafePath`, atomic symlink swap with backup/restore, cross‑FS degraded fallback (policy‑controlled), deterministic IDs and redaction.
  - Governance: preflight policy gates incl. rescue verification; locking semantics; smoke tests with auto‑rollback.
  - Observability: structured facts/audit emission with minimal provenance; backup retention and pruning APIs.

Status: early but functional. All commands are implemented; defaults are tuned for container/dev workflows. Production‑hardening guidance and presets are evolving.

### Stability and Safety Notes

- Defaults in `handler.rs` relax certain policy gates and disable smoke tests for container/dev ergonomics. Review and harden policy before making live changes with `--commit`.
- On a live root, commands may install/remove packages via pacman/AUR and update symlinks under `/usr/bin`. Ensure the system is quiescent (no pacman lock) and have a restore path.
- `replace` enforces parity gates suitable for a fuller switch; `use` activates replacements without parity enforcement.
- To enable production smoke checks and stricter governance, wire a smoke runner and enforce policy in your build/profile.

#### Why the defaults are relaxed (dev/CI rationale)

The defaults in `src/cli/handler.rs` are tuned for repeatable CI and container-based dev flows where the environment is intentionally minimal:

- allow_unlocked_commit = true
  - Rationale: our CI/container flows do not provision a system-wide lock manager path by default. Allowing commit without an external lock avoids spurious failures while still attaching a best-effort file lock.
- override_preflight = true
  - Rationale: preflight STOP gates (immutability, strict ownership, rescue, etc.) are useful in production but block experimentation in ephemeral containers. Overriding preflight lets us exercise apply/rollback paths during tests without requiring full system parity.
- rescue.require = false
  - Rationale: minimal CI images may not include BusyBox or enough GNU tools to meet the rescue profile. We still demonstrate backup/restore semantics in tests, but do not hard-fail on missing rescue capabilities.
- smoke = Off
  - Rationale: enabling smoke in containers can trigger auto‑rollback and add flakiness when binaries/symlinks are synthetic or when namespaces are unusual. Disabling smoke in dev keeps CI deterministic and focused on planner/apply parity.

These choices are about developer ergonomics—not about production guidance. When running on a live system with `--commit`, you should enable stricter governance.

#### Enabling production checks (example)

In production, enable locking, preflight gates, rescue verification, and smoke tests. For example:

```rust
use switchyard::api::ApiBuilder;
use switchyard::policy::{Policy, types::SmokePolicy};
use switchyard::adapters::{FileLockManager, DefaultSmokeRunner};
use switchyard::logging::JsonlSink;
use std::path::PathBuf;

let facts = JsonlSink::default();
let audit = JsonlSink::default();

let mut policy = Policy::production_preset();
policy.governance.allow_unlocked_commit = false;
policy.apply.override_preflight = false;
policy.rescue.require = true;
policy.governance.smoke = SmokePolicy::Required;

let api = ApiBuilder::new(facts.clone(), audit, policy)
    .with_lock_manager(Box::new(FileLockManager::new(PathBuf::from("/var/lock/switchyard.lock"))))
    .with_smoke_runner(Box::new(DefaultSmokeRunner::default()))
    .build();
```

See the Switchyard mdBook for more on presets and governance knobs.

## Packages

- coreutils → uutils-coreutils (pacman)
- findutils → uutils-findutils (AUR)
- sudo → sudo-rs (pacman)

## Build

```bash
# Compile
cargo build -p oxidizr-arch

# Help
cargo run -p oxidizr-arch -- --help
```

## Minimal CLI

```text
oxidizr-arch [--root PATH] [--commit] <COMMAND> [ARGS]
```

- `status` — report whether replacement symlinks are active
- `doctor` — Arch diagnostics (pacman lock, basic paths)
- `use` — ensure replacement installed and switch safely
- `restore` — switch back to GNU/stock
- `replace` — remove GNU packages after activating replacements

## CI on Ubuntu runners (Arch container)

GitHub Actions builds and runs a smoke test inside an Arch Linux container using `test-orch/docker/Dockerfile`.

Artifacts uploaded:

- `arch_status.json` from `status --json`
- `arch_doctor.json` from `doctor --json`

## Developer scripts

Convenience scripts live under `scripts/` in this crate:

- `scripts/arch_dev_shell.sh` — interactive Arch container with `oxidizr-arch` built from this repo, for safe manual testing.
- `scripts/arch_dev_proof.sh` — non-interactive proof run inside a disposable Arch container; prints diagnostics and JSON outputs.

These scripts do not modify your host; all operations occur inside a container.

## License

Apache-2.0 OR MIT
