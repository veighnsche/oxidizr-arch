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

## BDD / Gherkin Test Plan (deb, arch, cli-core)

This section tracks the work to update/create Gherkin tests across crates. Use `cucumber` with async `tokio` runner. Prefer common step glue where possible.

- __Conventions__
  - Directory layout per crate:
    - `features/` → `*.feature` files
    - `tests/bdd/` → `main.rs` cucumber runner and `steps.rs` glue
  - Tags:
    - `@unit-bdd` for hermetic tests (mock sets / temp roots; no PM mutations)
    - `@e2e` for containerized tests invoking real PM (run only in CI containers)
    - `@requires-paru` for Arch tests that require AUR helper
  - World state: temp root (via `tempfile`), captured outputs (stdout/stderr), last exit code, and paths.
  - Dev-deps (per crate as needed): `cucumber`, `tokio`, `assert_cmd`, `predicates`, `tempfile`, `serde_json`.

### oxidizr-cli-core (shared library)

- __[core-bdd-1]__ Feature: coverage preflight
  - Scenarios: complete coverage → OK; missing X → error lists X.
  - Glue: build fake "distro set" and "replacement set"; call `coverage_check()` and `coverage_preflight()`.

- __[core-bdd-2]__ Feature: resolve applets for use
  - Scenarios: live-root → intersects with distro; non-live → uses replacement allowlist/fallback.
  - Glue: stub `DistroAdapter` impl returning scripted lists; call `resolve_applets_for_use()`.

- __[core-bdd-3]__ Feature: discovery with allow
  - Scenarios: `--list` yields set; `--help` fallback; tiny set → static fallback kick-in.
  - Glue: simulate discovery by pointing to small helper binary or by mocking parser function (if exposed); otherwise validate `static_fallback_applets()` path.

- __[core-bdd-4]__ Harness
  - Add `tests/bdd/main.rs` runner and `tests/bdd/steps.rs` implementing world and steps.
  - Add `dev-dependencies` in `cargo/oxidizr-cli-core/Cargo.toml` as needed.

### oxidizr-deb (update existing/new tests)

- __[deb-bdd-1]__ Feature: dry‑run use coreutils
  - Ensure non-zero planned actions, no PM mutations, stable output keys.
  - Glue: run `oxidizr-deb use coreutils` in temp root; capture outputs; assert.

- __[deb-bdd-2]__ Feature: lock guard
  - Create dpkg lock file(s); `--commit use coreutils` fails closed with friendly message.
  - Glue: create lock under temp root path mirroring `/var/lib/dpkg/lock-frontend`.

- __[deb-bdd-3]__ Feature: replace coreutils coverage
  - With active use, ensure `replace coreutils` runs coverage preflight and reports missing if any; complete set proceeds to apt purge.
  - Tag `@e2e` for real apt in container; provide unit-bdd path with mocks.

- __[deb-bdd-4]__ Feature: restore with keep
  - `restore coreutils --keep-replacements` keeps RS installed but de-preferred; no purge invoked.

- __[deb-bdd-5]__ Feature: sudo setuid guard
  - `--commit use sudo` fails if `sudo-rs` lacks `root:root` and `4755`.

- __[deb-bdd-6]__ Glue & Runner
  - Add/refresh `tests/bdd/main.rs` and `tests/bdd/steps.rs`; implement reusable steps:
    - Given a staging root
    - When I run `oxidizr-deb ...`
    - Then exit code is N
    - And it emits `pm.install`/`pm.purge`/`pm.remove`
    - And path X resolves to replacement / original

### oxidizr-arch (new tests)

- __[arch-bdd-1]__ Feature: dry‑run use coreutils
  - Non-zero planned actions; no PM mutations; reports `pm.install` preview.

- __[arch-bdd-2]__ Feature: pacman lock guard
  - Create `/var/lib/pacman/db.lck`; `--commit use coreutils` fails closed.

- __[arch-bdd-3]__ Feature: replace coreutils coverage
  - With active use, `replace coreutils` requires 100% coverage; else error lists missing; on success runs `pacman -R`.

- __[arch-bdd-4]__ Feature: restore with/without keep
  - Ensures `pacman -S` for distro; optional removal of RS unless `--keep-replacements`.

- __[arch-bdd-5]__ Feature: sudo setuid guard
  - `--commit use sudo` fails if `sudo-rs` missing 4755 root:root.

- __[arch-bdd-6]__ Feature: AUR fallback (findutils)
  - Tag `@requires-paru`. If `paru` present: `use findutils` attempts `paru -S` when `pacman -S` 404s.
  - Provide unit-bdd variant with stubbed detection.

- __[arch-bdd-7]__ Glue & Runner
  - `tests/bdd/main.rs`, `tests/bdd/steps.rs`; shared steps with deb via a `tests/bdd_common/` module or pattern (optional now).
  - Reusable steps: same as deb, with pacman events (`pm.install`, `pm.remove`).

### CI Integration

- __[ci-bdd-1]__ GitHub Actions: add matrix jobs to run `cargo test -p oxidizr-cli-core --test bdd` for `@unit-bdd`.
- __[ci-bdd-2]__ Extend existing `oxidizr-arch-smoke` to run `@e2e` features inside Arch container (enable pacman and optional paru).
- __[ci-bdd-3]__ Add Debian-family container job to run `oxidizr-deb` `@e2e` features (apt available), separate from unit-bdd.
- __[ci-bdd-4]__ Upload cucumber JSON reports as artifacts; optionally convert to JUnit for CI surfaces.

### Implementation Notes

- Use `assert_cmd` to spawn CLIs with `--root <tempdir>`; set `PAGER=cat` for predictable output; capture stderr.
- For path resolution checks, compare `read_link()` or file content markers in hermetic roots as needed.
- Keep mutable tests behind `--commit`; ensure dry-run never mutates.
- Add a `bdd` feature flag if we need to gate dev-deps usage in release builds.

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
