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

Status: scaffolding only. `status` and `doctor` work; `use`/`replace`/`restore` are TODO.

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
- `use` — ensure replacement installed and switch safely (TODO)
- `restore` — switch back to GNU/stock (TODO)
- `replace` — remove GNU packages after activating replacements (TODO)

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
