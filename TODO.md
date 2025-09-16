# TODO — oxidizr-arch scaffolding → MVP

This file tracks the work to bring `cargo/oxidizr-arch` to parity with `cargo/oxidizr-deb`, adapted to Arch-based systems.

## Scope

- Packages (package-level UX only; no applets exposed):
  - coreutils → uutils-coreutils (pacman)
  - findutils → uutils-findutils (AUR)
  - sudo → sudo-rs (pacman)

## CLI surface (identical semantics)

- `use <package>` — ensure replacement installed (pacman/AUR), plan safe link swap via Switchyard, commit with `--commit`.
- `replace <package|--all>` — ensure replacement active, then remove GNU packages (pacman) under guardrails; always leave at least one provider.
- `restore <package|--all>` — restore GNU/stock binaries from backups; optionally `--keep-replacements` to keep RS packages installed but de‑preferred.
- `status` — report active state; `--json` output.
- `doctor` — environment diagnostics; Arch‑specific lock check and tips; `--json` output.

## Implementation plan

- Core
  - [ ] Implement `commands/use_cmd.rs` for Arch:
    - [ ] Live root: `pacman -S --noconfirm uutils-coreutils` (and `sudo-rs`) when needed.
    - [ ] AUR path (findutils): prefer `paru` if present; otherwise build/install `uutils-findutils` via available helper.
    - [ ] Non‑live roots (`--root != /`): require `--offline --use-local PATH` artifact; no pacman/AUR mutations.
    - [ ] Build Switchyard `PlanInput` to link applets → replacement binary under `DEST_DIR=/usr/bin` within `--root`.
    - [ ] Sudo preflight: enforce setuid root (4755) and owner root:root when committing.
  - [ ] Implement `commands/restore.rs` for Arch:
    - [ ] Generate `RestoreRequest`s to remove CLI‑managed links.
    - [ ] Ensure distro packages present via `pacman -S --noconfirm coreutils/findutils/sudo` on live root.
    - [ ] Post: if `--keep-replacements` is false, remove RS packages with `pacman -Rns --noconfirm`.
  - [ ] Implement `commands/replace.rs` for Arch:
    - [ ] Run `use` semantics first to guarantee replacement is active.
    - [ ] Verify invariants: replacement active; RS package installed.
    - [ ] Remove GNU packages with `pacman -Rns --noconfirm coreutils/findutils/sudo`.

- Adapters and preflight
  - [ ] `adapters/pacman.rs`: `pm_lock_message(--root)` detects `/var/lib/pacman/db.lck` and returns a helpful error when held.
  - [ ] `adapters/aur.rs`: helper to detect/install `paru` (or `yay`) with idempotent cache-friendly behavior. Reuse patterns from `test-orch` persistent cache logic.
  - [ ] `adapters/preflight.rs`: `sudo_guard(--root, path)` validates setuid root and owner root:root with `SafePath`.

- Fetch and artifacts (optional for MVP)
  - [ ] Support `--offline --use-local PATH` for injecting local unified binaries under a fakeroot.
  - [ ] (Future) Add basic artifact verification similar to `oxidizr-deb/src/fetch/`.

- Shared crate (dedupe with oxidizr-deb)
  - [x] Create `oxidizr-cli-core` with:
    - [x] `prompts::should_proceed(assume_yes, root)`
    - [x] `api::build_api(policy, lock_path)`
  - [ ] Consider moving these into shared as well:
    - [ ] `util::paths::ensure_under_root`
    - [ ] Common `status` implementation
    - [ ] Error types / result alias

- CI on Ubuntu runners (Arch container)
  - [x] Add `oxidizr-arch-smoke` job:
    - [x] Build `test-orch/docker/Dockerfile`
    - [x] Build `oxidizr-arch` inside the container
    - [x] Run `status --json` and `doctor --json` under hermetic root; upload artifacts
  - [ ] Expand smoke to `use coreutils --offline` with a local `uutils` binary once fetch/offline is wired.

## Flake and infra considerations

- pacman visibility for `sudo-rs` can be flaky on derivatives; mitigate per docs/Flakes/sudo-rs-not-found.md:
  - [ ] Retry with `pacman -Syy` then `pacman -Si sudo-rs` gating before install attempts.
- AUR helper availability varies; prefer `paru` and detect existing installs; fall back to building from AUR with idempotent directory logic (see `test-orch` improvements).
- Keep caches namespaced per distro (see `test-orch/host-orchestrator/dockerutil/dockerutil.go`) to avoid cross-container contention.

## Milestones

1. Minimal MVP: `status`, `doctor`, CLI skeleton compiles, CI smoke green. (DONE)
2. `use coreutils` live root with pacman; `restore coreutils` live root; hermetic roots supported via `--offline`. (NEXT)
3. AUR `use findutils` + `restore findutils`; invariant checks. 
4. `replace` flow with invariants and safe removal.
5. Dedupe shared utilities into `oxidizr-cli-core` as needed.
