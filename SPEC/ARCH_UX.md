# Arch UX Addendum (RFC-2119)

This addendum specifies CLI ergonomics and guardrails tailored for Arch Linux and derivatives. It complements
`cargo/oxidizr-arch/SPEC/SPEC.md` and inherits the engine invariants.

---

## 1. Distro Detection & Layout

- REQ-DIST-1: The CLI MUST detect the distro via `/etc/os-release` and record a `distro_id` fact (e.g., `arch`).
- REQ-DIST-2: The CLI MUST target `/usr/bin` for merged-/usr systems; on non-merged systems (rare), `/bin` is treated
  as a compatibility symlink and messaging SHOULD surface `/usr/bin` as the effective target.
- REQ-DIST-3: The CLI SHOULD detect `usrmerge` and surface a single effective target directory in messages.

Acceptance (pseudo):

- Given `/bin` is a symlink to `/usr/bin`, when `oxidizr-arch use coreutils` runs, then targets under `/usr/bin` are shown in the plan output.

---

## 2. Package-Manager Safety (pacman/paru)

- REQ-PKG-1: Before `apply`, the CLI MUST check for pacman locks and MUST stop with a friendly diagnostic when detected.
  Lock to check: `/var/lib/pacman/db.lck`.
- REQ-PKG-2: The CLI SHOULD recommend re-running after any ongoing pacman operation completes.
- REQ-PKG-3: Package manager operations are part of the high-level flows (`use`, `replace`, `restore`) and occur in the CLI layer
  before and/or after the engine `plan/preflight/apply` phases. The engine steps themselves MUST NOT invoke `pacman` or `paru`.
  There are no standalone package-manager-only commands. In dry-run, no mutations occur; the CLI SHOULD print the exact
  command(s) it would run (e.g., `pacman -S --noconfirm ...`, `pacman -R --noconfirm ...`, `paru -S --noconfirm ...`).
- REQ-PKG-4: Package manager operations MUST run against the live system root (`--root=/`). When `--root` points to a
  non-/ path, PM commands MUST refuse with guidance (running inside a chroot with pacman configured is acceptable but
  out of the current scope).

Acceptance:

- Given pacman is running and holds `/var/lib/pacman/db.lck`, when I run `oxidizr-arch --commit ...`, then it fails with a message: "Package manager busy (pacman db.lck detected); retry after current operation finishes." and exits non-zero without mutating.

---

## 3. Provider Availability Invariants (Coreutils & Sudo)

- REQ-AV-1: At all times there MUST be at least one functional provider of `coreutils` and one provider of `sudo`
  installed. The CLI MUST refuse any `replace` or `restore` removal step that would leave zero providers.
- REQ-AV-2: The CLI MUST verify provider counts before and after PM operations and abort when invariants would be
  violated.

Acceptance:

- Given only `coreutils` is installed and `uutils-coreutils` is not installed, when I run `oxidizr-arch --commit replace coreutils`, then the command fails with an invariant error and no PM changes are performed.

---

## 4. Replace (Removal) UX

- REQ-REP-1: The CLI MUST provide a `replace <package|all>` command to remove legacy distro packages only after the
  replacement is installed, active, and healthy (based on the last committed run and smoke status). If not active, `replace`
  MUST perform `use` semantics first and then proceed.
- REQ-REP-2: `replace` MUST respect pacman locks (see §2). If locks are present, it fails closed with a friendly diagnostic
  and does not invoke any package manager tools.
- REQ-REP-3: `replace` MUST require explicit confirmation when a TTY is present unless `--assume-yes` is set.
- REQ-REP-4: `replace` MUST use distro tools (`pacman -R`) to remove legacy packages. It MUST propagate
  exit codes and SHOULD capture a short stderr summary.
- REQ-REP-5: `replace` MUST emit a structured CLI event (not an engine fact) with fields: `pm.tool`, `pm.args`, `exit_code`,
  `stderr_tail`, and `package`. Logs MUST NOT contain secrets.
- REQ-REP-6: Dry‑run MUST NOT execute any package manager mutations and SHOULD print the exact command that would run.
- REQ-REP-7 (No Missing Commands): Before removing distro packages for `coreutils` (and analogously for `findutils`), the CLI MUST ensure that every command provided by the distro package remains present after commit and resolves to a functional provider. The CLI MUST enumerate distro-provided commands (e.g., `pacman -Ql coreutils`) and intersect with the replacement’s supported applets (interrogated from the unified binary). If coverage is incomplete, `replace` MUST stop with a clear error listing missing commands; no partial state with missing commands is permitted.
- REQ-REP-8 (Post Verification): After removal, the CLI MUST verify zero missing commands and abort/report failure otherwise; the engine’s rollback semantics apply.

Acceptance:

- Given coreutils is active and healthy and no locks are present, when I run `oxidizr-arch --commit replace coreutils`, then `pacman -R --noconfirm coreutils` is invoked and the command exits 0.
- Given `pacman -Ql coreutils` lists command names A..Z and the replacement reports a supported set that includes A..Z, when I run `oxidizr-arch --commit replace coreutils`, then every A..Z resolves to the replacement after commit (no missing commands), and verification passes.
- Given `pacman -Ql coreutils` lists a command M that the replacement does not support, when I run `oxidizr-arch --commit replace coreutils`, then the command fails closed with an error listing `M` as missing and no PM mutations are performed.
- Given locks are present, when I run `oxidizr-arch replace coreutils --commit`, then the command exits non-zero without invoking `pacman` and prints a lock diagnostic.

---

## 5. Alternatives / Diversions (Implementation Notes)

- Arch does not rely on `dpkg-divert`. The CLI uses direct symlink topologies via Switchyard. An alternatives-like mechanism MAY be considered in the future behind a feature flag; `restore` MUST undo any topology it created.

---

## 6. sudo Package Hardening

- REQ-SUDO-1: Before commit, the replacement binary for `sudo` MUST be `root:root` and `mode=4755` (setuid root).
- REQ-SUDO-2: If not satisfied, preflight MUST STOP with an error and a human-readable remediation.

Acceptance:

- Given the replacement `sudo-rs` is not setuid root, when I run `oxidizr-arch --commit use sudo`, then preflight fails closed with an explanation about setuid ownership/mode.

---

## 7. Coreutils and Findutils Package Ergonomics

- REQ-CU-1: Applet selection MUST NOT be exposed in the CLI; mappings to unified binaries are internal and complete from the operator’s perspective. After commit, all distro-provided command names remain present and resolve to a provider. The CLI dynamically resolves/links the applet set appropriate for the current system (live root: pacman-derived).
- REQ-CU-2: After commit, the CLI SHOULD print a short "Next steps" hint that references `oxidizr-arch --commit replace <package>` to remove legacy packages safely under guardrails.

Acceptance:

- After a successful coreutils or findutils commit, the CLI prints a safe reminder, e.g.: "Next steps: when confident, run 'oxidizr-arch --commit replace coreutils' to remove legacy packages; see --help for rollback." (wording may vary but MUST include a safe reminder).

---

## 8. Prompts & Non-Interactive Modes

- REQ-UX-1: On `--commit`, the CLI SHOULD present a summary prompt (unless `--assume-yes`) showing N planned actions and affected directories.
- REQ-UX-2: `--assume-yes` MUST suppress prompts for batch use.
- REQ-UX-3: Dry-run output SHOULD be parsable (stable keys and order) to support wrappers.

Acceptance:

- Running with `--commit` interactively shows a confirmation that includes the number of actions and top-level target dirs.

---

## 9. Output Conventions & Diagnostics

- REQ-OUT-1: Error messages MUST include stage context (preflight/apply/pm) and one-line cause.
- REQ-OUT-2: Arch-specific tips SHOULD accompany common failures:
  - Pacman lock → tip to wait and retry.
  - Missing setuid on `sudo` → tip to verify `chown root:root` and `chmod 4755` on the replacement binary.

---

## 10. Smoke Test Extensions (Packages)

- REQ-SMOKE-1 (sudo): Minimal additional checks SHOULD verify owner/mode (no execution of `sudo` required).
- REQ-SMOKE-2 (coreutils): Optional checks MAY validate that representative commands resolve to the unified binary (mapping remains internal).

---

## 11. Documentation & Completions

- REQ-DOC-1: The CLI SHOULD offer shell completion generation (bash/zsh/fish) via a dedicated command.
- REQ-DOC-2: The CLI MAY offer a manpage generator. Distributions MAY package prebuilt manpages.

---

## 12. Safety Boundaries

- REQ-SAFE-1: The engine `plan/preflight/apply` MUST NOT invoke package manager operations. PM operations are allowed only
  as part of the high-level flows (`use`, `replace`, `restore`) with guardrails (locks, confirmations, dry‑run safety) and the
  live-root constraint.
- REQ-SAFE-2: All swapped paths MUST be within `--root`. No out-of-root writes.

---

## 13. Fetching & Verification (Arch specifics)

- REQ-FETCH-A-1: For `use <package>` and `replace <package>`, the CLI MUST ensure the appropriate replacement package for the
  distro and architecture is installed via pacman or paru.
- REQ-FETCH-A-2: Integrity and provenance MUST rely on the package manager’s signature verification and repository trust.
  Offline/manual artifact injection is NOT part of acceptance proof; acceptance must exercise pacman/paru.

---

## 14. Operator tooling (dev shell)

- REQ-DEV-1: The repository SHOULD provide a helper to launch a disposable Arch shell with replacements applied on the
  live root inside the container via the CLI using pacman/paru, so an operator can manually verify versions (e.g., `ls --version`).
  The helper MUST NOT mutate the host system.

---

## 15. Updates & Upgrades (Arch specifics)

- REQ-UPDATE-A-1: On mutating runs of `use <package>` or `replace <package>`, the CLI MUST ensure the corresponding rust
  replacement package is installed and upgraded to the latest available version via pacman/paru (latest by default; future
  version pinning is out of scope here).
- REQ-UPDATE-A-2: `status` and `doctor` SHOULD surface when a replacement package is outdated to guide operators to run an
  update via the high-level flows.
