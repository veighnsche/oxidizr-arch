# oxidizr-arch User Flows (Arch Linux)

This file documents end-to-end user flows for oxidizr-arch. It reflects the updated SPEC where the CLI manages both
replacement and distro packages via pacman (and optionally paru for AUR), and uses Switchyard for safe filesystem swaps.

---

## Guardrails and Invariants (Always On)

- At least one provider of coreutils and one provider of sudo must always be installed.
  - Providers (coreutils): GNU `coreutils` or `uutils-coreutils` (replacement).
  - Providers (sudo): GNU `sudo` or `sudo-rs` (replacement).
- Package manager operations require the live root (`--root=/`).
- Package manager locks abort operations with a friendly message (pacman lock: `/var/lib/pacman/db.lck`).
- Dry-run never mutates: the CLI prints the exact pacman/paru command(s) it would run.
- The CLI exposes only three high-level operations: `use`, `replace`, and `restore`. There are no standalone package-manager-only commands; installs/removals happen inside these flows.
- Engine-backed swaps (`use`/`replace`/`restore`) enforce SafePath, TOCTOU-safe operations, backups + rollback, and minimal smoke tests.

---

## Quick Command Cheat‑Sheet

- Use replacements (installs if needed; makes them preferred): `oxidizr-arch --commit use <target>`
- Replace distro with replacements (installs if needed; makes them preferred; removes distro packages): `oxidizr-arch --commit replace <target>`
- Restore to distro (reinstalls GNU packages if needed; makes them preferred). By default removes RS packages; keep them with `--keep-replacements`: `oxidizr-arch --commit restore <target> [--keep-replacements]`
- Status / Doctor: `oxidizr-arch status`, `oxidizr-arch doctor`

`<target>` ∈ { `coreutils`, `findutils`, `sudo` }

Note: Mutating flows (`use`, `replace`) ensure the relevant rust replacement packages are installed and upgraded to the latest available version via pacman or paru.

---

## Flow 1 — Switch coreutils to the latest uutils-coreutils (safe swap)

1) Preview (dry‑run):
   - `oxidizr-arch use coreutils`
   - Ensures `uutils-coreutils` can be installed (will be installed during commit if missing).
   - Prints planned action count; no changes.

2) Commit:
   - `oxidizr-arch --commit use coreutils`
   - Pre-checks pacman lock; confirms (unless `--assume-yes`).
   - Installs/updates `uutils-coreutils` (latest) via pacman, or `paru` if package is AUR-only and `paru` is available.
   - Switchyard plan → preflight → apply to set the symlink topology under `/usr/bin` with backups.
   - Runs minimal smoke tests; auto‑rollback on failure; exits non‑zero with diagnostics if failed.

3) Verify:
   - `oxidizr-arch status` shows `coreutils: active`.

---

## Flow 2 — Replace coreutils (remove GNU `coreutils`)

1) Preconditions:

- `coreutils` is active (symlinks point to uutils).
- pacman is not holding locks.

2) Command:
   - `oxidizr-arch --commit replace coreutils`

3) Behavior:

- Confirms (unless `--assume-yes`).
- Ensures `uutils-coreutils` is installed/updated and preferred (performs "use" semantics if not already active).
- Verifies availability invariant will still hold (rust replacement remains installed).
- Performs coverage preflight (no missing applets compared to `pacman -Ql coreutils`).
- Runs `pacman -R --noconfirm coreutils` and emits a `pm.remove` event with tool/args/exit code/stderr tail.
- Post-apply verifies zero missing commands.

---

## Flow 3 — Restore coreutils to GNU (with optional keep)

1) Command:

- `oxidizr-arch --commit restore coreutils [--keep-replacements]`

2) Behavior:

- Ensures GNU `coreutils` is installed (installs if missing) and makes it preferred: Switchyard restores backups and removes CLI‑managed symlinks to reinstate the prior GNU topology.
- By default, removes the replacement package (`uutils-coreutils`) via pacman; if `--keep-replacements` is provided, keeps it installed but de‑preferred.
- Runs minimal smoke tests; auto‑rollback on failure; exits non‑zero with diagnostics if failed.

---

## Flow 4 — sudo specifics

- On `use sudo`, preflight requires the replacement binary (`sudo-rs`) to be `root:root` and mode `4755` (setuid root) before commit on live root.
- Availability invariant applies: cannot remove both `sudo` and `sudo-rs`.
- Replace for sudo removes `sudo` after `sudo-rs` is active and healthy: `oxidizr-arch --commit replace sudo`.

---

## Flow 5 — Diagnostics and health

- `oxidizr-arch status [--json]` — shows active states for `coreutils`, `findutils`, `sudo`.
- `oxidizr-arch doctor [--json]` — checks common issues (paths, locks) and prints tips.

---

## Flow 6 — Common failure cases and resolutions

- Pacman lock present
  - Symptom: "Package manager busy (pacman db.lck detected)".
  - Action: Wait for pacman to finish; retry.

- Invariant violation (last provider removal attempted)
  - Symptom: Refusal to remove package; message indicates it would leave zero providers.
  - Action: Run `oxidizr-arch --commit restore <target>` to ensure the GNU package is installed and preferred; optionally add `--keep-replacements` if you want RS packages to remain installed but de‑preferred.

- Smoke test failure after `use`
  - Symptom: `--commit use` fails, auto‑rollback triggers.
  - Action: Inspect logs/facts; resolve compatibility issues; optionally run `doctor`. If using `replace`, the same guidance applies.

- Non‑live root (`--root` not `/`) for PM operations
  - Symptom: PM command refuses and prints guidance.
  - Action: Run on the live system (or inside a chroot with pacman configured; currently out of scope).

---

## Notes and Future Extensions

- Version pinning: CLI may allow `--version` for install commands in the future; latest remains default.
- Alternatives/diversions: not used on Arch by default; `restore` undoes the Switchyard symlink topology.
- Telemetry: PM operations emit CLI-level structured events (pm.install/remove).
