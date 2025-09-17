# oxidizr-arch Specification (RFC-2119)

## 0. Domain & Purpose

oxidizr-arch is an Arch Linux CLI that orchestrates safe, atomic, reversible filesystem swaps
(e.g., GNU coreutils/findutils → uutils-coreutils/uutils-findutils; sudo → sudo-rs) with a simple package-level UX.

- oxidizr-arch is a thin CLI front-end to the Switchyard engine. All filesystem mutations are delegated to
  Switchyard via `plan → preflight → apply`; the CLI composes inputs, configures adapters, and handles
  Arch package lifecycle tasks via pacman (and optionally paru for AUR).
- Users do not choose applets, sources, or targets. The CLI provides a high-level `use` command per package
  and executes a safe plan under the hood.
- Replacement and distro packages are managed via the system package manager (pacman; optionally AUR via paru).
  The CLI ensures installation and removal happen as part of the high-level flows (`use`, `replace`, `restore`) —
  not as standalone commands. It installs the appropriate replacement packages (`uutils-coreutils`, `uutils-findutils`, `sudo-rs`)
  as needed and ensures distro packages (`coreutils`, `findutils`, `sudo`) are present when restoring, under guardrails.
- The CLI is responsible for the full package lifecycle under these flows: install/upgrade when enabling, removal when
  replacing, and re-installation when restoring; all under an availability invariant.

---

## 1. Main Guarantees

The CLI inherits the engine’s guarantees and adds CLI-specific guardrails. Unless explicitly stated otherwise
below, the engine’s invariants (SafePath boundaries, TOCTOU-safe syscalls, rollback on failure, deterministic plans)
apply transitively to oxidizr-arch.

- Atomic, crash-safe swaps with backups and no user-visible broken/missing path.
- Complete, idempotent rollback on mid-plan failure (engine-managed).
- `SafePath` is enforced at CLI boundaries; all mutating inputs are validated.
- Deterministic plans and outputs via the engine (stable IDs; dry-run redactions stabilized).
- Production locking enabled by default via a filesystem lock under `<root>/var/lock/oxidizr-arch.lock`.
- Minimal smoke tests are run post-apply under production presets; failure triggers auto‑rollback unless disabled by policy.
- Dry‑run is the default mode; side effects require `--commit`.
- Cross-filesystem safety follows the package policy (built-in packages disallow degraded mode by default).

---

## 2. Normative Requirements

### 2.1 CLI Construction & SafePath Boundaries

- REQ-C1: The CLI MUST accept a `--root` argument (default `/`) and construct all mutating paths using a SafePath
  boundary rooted at `--root`. Any failure to validate MUST abort the command with an error message.
- REQ-C2: The CLI MUST NOT pass unvalidated filesystem paths to mutating engine APIs.
- REQ-C3: The `root` argument MUST be absolute.

### 2.2 Modes & Conservatism

- REQ-M1: The CLI MUST default to dry‑run. A user MUST supply `--commit` to perform mutations.
- REQ-M2: On dry‑run, the CLI SHOULD emit a clear summary (e.g., planned action counts).
- REQ-M3: On failure, the CLI MUST not leave the system in a partially applied state; automatic reverse‑order
  rollback semantics apply.

### 2.3 Locking (Process Serialization)

- REQ-L1: The CLI MUST configure process serialization with bounded wait. Default lock path:
  `<root>/var/lock/oxidizr-arch.lock`.
- REQ-L2: When the active policy requires locking, absence of a lock MUST cause apply to fail rather than proceed concurrently.

### 2.4 Packages & Implicit Policies

- REQ-PKG-1: `use coreutils` and `use findutils` MUST apply implicit policies tuned for their link topologies,
  including disallowing degraded cross‑filesystem fallback (EXDEV → fail), strict ownership and preservation where
  applicable, and forbidding untrusted sources.
- REQ-PKG-2: `use sudo` MUST apply a production‑grade policy tuned for replacing `/usr/bin/sudo` safely.
- REQ-PKG-3: Cross‑filesystem degraded fallback MUST be disallowed by default for all built‑in packages.

### 2.5 Health Verification

- REQ-H1: Under commit, the CLI MUST run minimal post‑apply smoke checks appropriate to the package and obey the
  policy’s `require_smoke_in_commit` behavior: smoke failure triggers auto‑rollback and an error.

### 2.6 Observability & Audit

- REQ-O1: The CLI MUST initialize audit/facts sinks for the engine.
- REQ-O2: When compiled with file‑logging support, deployments MAY configure file‑backed JSONL sinks; otherwise a
  no‑op sink satisfies development usage.
- REQ-O3: The CLI MUST NOT emit secrets in its own logs; redaction is enforced by the engine’s sinks.
- REQ-O4: Package manager operations performed by the CLI (e.g., `pacman -S/-R`, or `paru -S`) SHOULD be
  logged as structured CLI events (e.g., `pm.install`, `pm.remove`) including tool, args, exit code, and
  stderr summary. These are CLI-level logs and do not alter the engine’s audit facts schema.

### 2.7 Error Reporting & Exit Codes

- REQ-E1 (v0): The CLI MUST exit `0` on success and `1` on error. A future revision SHOULD align exit codes with
  a published error taxonomy.
- REQ-E2: User‑facing error messages SHOULD include the failing stage (preflight/apply/pm) and a brief cause.

### 2.8 Filesystems & Degraded Mode

- REQ-F1: Cross‑filesystem behavior is governed by the package’s implicit policy. Degraded fallback MUST be disallowed
  by default (apply fails with a stable reason; no visible change).

### 2.9 Retrieval & Versioning (pacman/paru)

- REQ-RV-1: `use <package>` MUST ensure the appropriate replacement package is installed via pacman when available.
  If not available in official repos, the CLI MAY try `paru -S` when present.
- REQ-RV-2: Integrity and provenance SHOULD rely on the package manager’s signature verification and repository trust.
- REQ-RV-3: Offline/manual artifact injection is not part of acceptance proof.
- REQ-RV-4: Future versions MAY allow pinning a specific version via flags on `use`/`replace`; latest remains the default.

### 2.10 Persistence & Cleanup

- REQ-PERM-1: After a successful `use <package>` commit, the CLI MUST keep the user’s selection active across
  package upgrades; no extra user steps are required.
- REQ-CLEAN-1: After a successful `restore <package>`, the CLI MUST remove cached replacement artifacts for the
  restored package to avoid clutter.

### 2.11 Package Lifecycle via Package Manager & Availability Invariants

- REQ-PM-1 (Install replacements): Under `use` or `replace`, the CLI MUST ensure replacement packages are
  installed/upgraded via pacman/paru (`uutils-coreutils`, `uutils-findutils`, `sudo-rs`).
- REQ-PM-2 (Replace removes distro): Under `replace`, once replacements are active and healthy, the CLI MUST remove
  the corresponding distro packages via pacman.
- REQ-PM-3 (Restore ensures distro and handles replacements): Under `restore`, the CLI MUST (re)install the distro
  packages if missing and make them preferred. By default, it MUST remove the replacement packages; when
  `--keep-replacements` is provided, it MUST keep them installed but de‑preferred.
- REQ-PM-4 (Locks): Before any package manager mutation, the CLI MUST check for pacman locks (e.g., `/var/lib/pacman/db.lck`) and fail closed with a
  friendly diagnostic when locks are present.
- REQ-PM-5 (Dry‑run): In dry‑run, the CLI MUST NOT execute package manager mutations and SHOULD print the exact
  command(s) that would run.
- REQ-PM-6 (Confirmations): When a TTY is present and `--assume-yes` is not set, the CLI MUST require explicit
  confirmation before invoking install/remove.
- REQ-PM-7 (Availability invariant): At all times there MUST be at least one functional provider of `coreutils`
  and one functional provider of `sudo` installed. The CLI MUST refuse any operation that would leave zero
  providers (e.g., removing both `coreutils` and `uutils-coreutils`, or removing `sudo` and `sudo-rs`).
- REQ-PM-8 (Pre/Post checks): The CLI MUST verify provider counts before and after package manager operations and
  abort/rollback when invariants would be violated.

### 2.12 Updates & Ongoing Maintenance

- REQ-UPD-1: On any mutating run of `use <package>` or `replace <package>`, the CLI MUST ensure the corresponding
  rust replacement package is installed and upgraded to the latest available version via pacman/paru (unless version pinning is supported later; latest remains default).
- REQ-UPD-2: The CLI SHOULD surface when a replacement package is out of date in `status` and `doctor` output to
  guide operators to run an update via the high-level flows.

### 2.13 Replacement Coverage & No Missing Commands

- REQ-COVER-1: Under `replace coreutils` (and analogously for `findutils`), the CLI MUST guarantee that every command
  provided by the distro package under `/usr/bin` (and legacy `/bin`) remains present and resolves to the replacement after
  commit. There MUST NOT be any missing commands or dangling/missing symlinks.
- REQ-COVER-2: Before removing the distro package, the CLI MUST perform a coverage preflight by enumerating the
  distro-provided command set (e.g., `pacman -Ql coreutils`) and intersecting it with the replacement’s supported
  applet set (interrogated from the unified binary). If coverage is incomplete, `replace` MUST stop with a clear
  error that lists the missing commands. No filesystem mutation may leave the system partially missing commands.
- REQ-COVER-3: Under `use <package>`, the CLI MUST link all applets supported by the replacement for the current
  system. On live roots, it MUST intersect with the distro-provided set to avoid stray/nonexistent targets.
- REQ-COVER-4: Post-apply verification under `replace` MUST assert zero missing commands before reporting success.
  If verification fails, the CLI MUST abort and rely on the engine’s rollback to preserve prior availability.

Acceptance hints:

- Given `pacman -Ql coreutils` lists N commands under `/usr/bin`, when `oxidizr-arch --commit replace coreutils` completes,
  then for each listed command, the path exists and resolves to the replacement provider, and no missing entries are observed.

---

## 3. Public Interfaces (CLI)

### 3.1 Synopsis

```
oxidizr-arch [--root PATH] [--commit] <COMMAND> [ARGS]
```

Global options:

- `--root PATH` — absolute root of the filesystem tree (default `/`).
- `--commit` — commit changes; without it, commands run in dry‑run.

### 3.2 Commands

- use
  - Arguments: `<package>` where `<package>` ∈ {`coreutils`, `findutils`, `sudo`} (extensible).
  - Semantics: ensures the appropriate replacement package is installed via pacman/paru (installing/upgrading to latest if
    needed), then plans and applies a safe link topology with backups using Switchyard. No applet selection is exposed;
    mappings are internal.
- replace
  - Arguments: `<package|all>`.
  - Semantics: ensures the appropriate replacement package is installed and active; then removes the
    corresponding distro packages via pacman under guardrails. Performs `use` semantics first if not already active.
- restore
  - Arguments: `<package|all>`; flags: `--keep-replacements` to keep RS packages installed but de‑preferred.
  - Semantics: restores GNU/stock tools for the package (or all) from backups and ensures distro packages are installed
    and preferred. By default removes RS packages; with `--keep-replacements`, keeps them installed but de‑preferred.
- status
  - Arguments: none.
  - Semantics: reports current active/linked state and what can be restored.
- doctor
  - Arguments: `--json` (optional).
  - Semantics: runs environment checks (paths, locks) and outputs a summary; non-mutating.
- completions
  - Arguments: `<shell>` where `<shell>` ∈ {`bash`,`zsh`,`fish`}.
  - Semantics: generates shell completions.

Engine-backed file operations within `use`, `replace`, and `restore` execute `plan → preflight → apply` through Switchyard
and honor policy gates. Package‑manager steps are orchestrated by the CLI before/after engine phases; the engine itself
never invokes pacman/paru.

---

## 4. Preflight Diff & Audit Facts

oxidizr-arch reuses the engine’s schemas without modification.

- Preflight Diff schema: see engine SPEC.
- Audit Facts schema: see engine SPEC.

Dry‑run outputs are byte‑identical to real‑run (after redactions) and follow deterministic ordering.

---

## 5. Filesystems & Degraded Mode (Operational Guidance)

- Coreutils and Findutils: degraded cross‑filesystem fallback is disallowed by default; EXDEV causes apply to fail with a
  stable reason marker; no visible changes occur.
- Sudo: degraded fallback is disallowed by default.

---

## 6. Acceptance Tests (CLI-flavored BDD)

```gherkin
Feature: Safe swaps via CLI (Arch)
  Scenario: Dry-run use of coreutils
    Given a staging root at /tmp/fakeroot
    When I run `oxidizr-arch use coreutils`
    Then the command exits 0
    And it reports a dry-run with a non-zero planned action count

  Scenario: Commit sudo use
    Given pacman/paru can install sudo-rs
    When I run `oxidizr-arch --commit use sudo`
    Then the command exits 0
    And subsequent reads of /usr/bin/sudo resolve to the rust replacement

  Scenario: Use and restore findutils
    Given a staging root at /tmp/fakeroot
    And pacman/paru can install uutils-findutils
    When I run `oxidizr-arch --commit use findutils`
    Then the command exits 0
    And representative findutils commands resolve to the rust replacement
    When I run `oxidizr-arch restore findutils`
    Then the command exits 0
    And the original binaries are restored

  Scenario: Restore package
    Given backups exist for coreutils
    When I run `oxidizr-arch restore coreutils`
    Then the command exits 0
    And the original binaries are restored

  Scenario: Make permanent for coreutils
    Given coreutils is active and smoke checks have passed
    And no pacman lock is present
    When I run `oxidizr-arch --commit replace coreutils`
    Then the command exits 0
    And the legacy `coreutils` package is removed via pacman
    And a structured CLI event is emitted with the outcome

  Scenario: Use ensures replacement installed
    Given `uutils-coreutils` is not installed
    And pacman is not locked
    When I run `oxidizr-arch --commit use coreutils`
    Then `pacman -S --noconfirm uutils-coreutils` or `paru -S --noconfirm uutils-coreutils` is invoked
    And the command exits 0
    And a `pm.install` event is emitted

  Scenario: Availability guard prevents removing last coreutils provider
    Given only `coreutils` (GNU) is installed and `uutils-coreutils` is not installed
    When I run `oxidizr-arch --commit replace coreutils`
    Then the command fails with an invariant error
    And no package manager changes are performed

  Scenario: Restore ensures distro present
    Given `coreutils` is not installed
    And pacman is not locked
    When I run `oxidizr-arch --commit restore coreutils`
    Then `pacman -S --noconfirm coreutils` is invoked
    And the command exits 0
```

---

## 7. Operational Bounds

- Default lock file: `<root>/var/lock/oxidizr-arch.lock`.
- All operations are scoped to `--root`.
- Plan sizes and performance bounds are inherited from the engine.

---

## 8. Security Requirements Summary (CLI)

- Enforce SafePath at boundaries and reject unsafe paths.
- Dry‑run by default; explicit `--commit` required.
- Locking configured by default under a predictable path; bounded wait.
- Minimal smoke checks post‑apply; failure triggers auto‑rollback unless disabled by policy.
- Cross‑filesystem degraded mode disallowed by default for built‑in packages.
- Explicit `replace` command for safe removal of legacy packages under guardrails.

---

## 9. Versioning & Future Work

- v0 CLI exits 0 on success, 1 on error. Future versions SHOULD align exit codes with a published taxonomy
  and surface specific error identifiers.
- Future flags MAY expose policy toggles (e.g., degraded fallback, rescue thresholds, retention pruning), provided
  they continue to enforce SafePath and engine invariants.

---

## 10. Arch UX Addendum

For Arch-focused ergonomics (pacman lock detection, optional AUR helper via paru, sudo setuid checks, prompts,
completions, and diagnostics), see `cargo/oxidizr-arch/SPEC/ARCH_UX.md`. These requirements complement this SPEC and are
normative where marked with RFC‑2119 keywords.
