# BUG ANALYSIS: Arch container proof not green (post-rerun)

## Overview

We reran the full test suite and the Arch proof with the harness updated to comply with the testing policy (CLI is responsible for installing replacements; the harness only ensures infra like an AUR helper is present).

Findings:

- The Switchyard test suite is green across the board.
- The Arch proof shows the CLI performing installs and executing symlink actions, but coreutils activation still produces broken applets due to an incorrect `source_bin` path selection on Arch.
- AUR-based findutils installation fails when the CLI invokes `paru` as root.

## Reproduction

- Script: `scripts/arch_dev_proof.sh`
- Container image: `archlinux:base-devel`
- Steps inside the container now follow policy:
  - Build and install `oxidizr-arch` from the workspace.
  - Ensure an AUR helper (`paru`) exists (infra only; allowed per `docs/testing/TESTING_POLICY.md`).
  - Run:
    - `oxidizr-arch doctor --json`
    - `oxidizr-arch status --json`
    - `oxidizr-arch --commit use coreutils`
    - `oxidizr-arch --commit use findutils`
    - `oxidizr-arch --commit use sudo`
    - `oxidizr-arch status --json`

## Fresh run results

- Switchyard tests via `python3 test_ci_runner.py`: all green (129+ tests ok).
- Proof run key events (CLI JSON logs and shell diagnostics):
  - Coreutils:
    - `{"event":"pm.install","tool":"pacman","package":"uutils-coreutils","exit_code":0}`
    - `{"event":"use.exec.resolved","package":"Coreutils","source_bin":"/usr/lib/uutils-coreutils/uutils","applets_count":104}`
    - `{"event":"use.exec.apply_ok","executed_actions":104}`
    - After apply: `/usr/bin/ls` is a symlink to `/usr/lib/uutils-coreutils/uutils`, but `exists(target)=false` and `exec(target)=false`.
    - Subsequent script step using `tee` failed with `tee: command not found` (expected when many applets link to a non-existent binary).
  - Findutils:
    - `{"event":"pm.install","tool":"pacman","package":"uutils-findutils-bin","exit_code":1,"stderr_tail":"error: target not found: uutils-findutils-bin"}`
    - `{"event":"pm.install","tool":"paru","package":"uutils-findutils-bin","exit_code":1,"stderr_tail":"error: can't install AUR package as root"}`
    - CLI aborts findutils install with a clear error.
  - Sudo:
    - `{"event":"pm.install","tool":"pacman","package":"sudo-rs","exit_code":0}`
    - `{"event":"use.exec.apply_ok","executed_actions":1}`

## Root causes

- __Incorrect `source_bin` path on Arch for coreutils__
  - File: `cargo/oxidizr-arch/src/commands/use_cmd.rs` → `resolve_source_bin()` currently prefers `"/usr/lib/uutils-coreutils/uutils"` when it exists. On this Arch image, pacman installed `uutils-coreutils` but did not place a dispatcher at that path, leaving our symlinks dangling.
  - Most likely correct locations on current Arch are `"/usr/bin/coreutils"` or `"/usr/bin/uutils"` (package layout can vary across versions). We must detect rather than guess.

- __AUR install under root for findutils__
  - The CLI tries `pacman` first (correct), and then `paru` if present. `paru` refuses to run as root by design. Running the CLI as root (required to mutate `/usr/bin`) therefore makes `paru` unusable unless we delegate to a non-root user.
  - Our harness now complies with the policy and does not pre-install replacements. Therefore, the CLI must either:
    - gracefully instruct the user to install `uutils-findutils-bin` using their AUR helper as a non-root user (then rerun `use`), or
    - support a configurable non-root AUR helper user to run the helper under (requires design), or
    - ship/find an official repo package path instead.

- __Status heuristic fragility (secondary)__
  - `cargo/oxidizr-arch/src/commands/status.rs` considers coreutils active if any of `ls|cat|echo|mv` are symlinks. With dangling targets, these are broken, so `status` remains `unset`. Even after we fix the dispatcher path, aligning `status` to the actual resolved applets (or offering a verbose mode) will improve determinism.

## Concrete fixes

- __Robust Arch dispatcher detection__
  - Update `resolve_source_bin()` to detect the installed dispatcher by querying pacman when the replacement package is installed:
    - `pacman -Ql uutils-coreutils | grep -E '/(uutils|coreutils)$'` and select the actual file path.
    - Fallback order for coreutils if pacman query fails: `/usr/bin/coreutils`, `/usr/bin/uutils`, then `/usr/lib/uutils-coreutils/uutils`.
    - For findutils: query `uutils-findutils-bin` similarly; fallback: `/usr/bin/findutils`, `/usr/lib/uutils-findutils/findutils`, `/usr/bin/uutils`.

- __AUR helper under root__
  - Short-term: when running as root and `paru` denies execution, emit a clear, actionable error:
    - "AUR helper 'paru' refuses to run as root. Install 'uutils-findutils-bin' with your AUR helper as a non-root user, then rerun: `oxidizr-arch --commit use findutils`."
  - Medium-term design (optional): support `--aur-helper-user <name>` so the CLI can run `sudo -u <name> paru -S uutils-findutils-bin` while the CLI itself still runs as root for the symlink apply.

- __Proof script resiliency (non-blocking for product)__
  - Avoid using coreutils applets after apply if the dispatcher could be mis-resolved. Replace `tee` with shell redirection to capture JSON, which we already do in other places.

- __Status improvements (optional)__
  - Base `status` on the applets actually resolved for the package (or provide a `--verbose` mode that enumerates linked applets and validates their targets against the current dispatcher path).

## Acceptance criteria

- After fixes, running `scripts/arch_dev_proof.sh` should show:
  - Coreutils: `use.exec.apply_ok` with `executed_actions > 0`, `/usr/bin/ls` points to an existing executable dispatcher, `status.coreutils == "active"`.
  - Sudo: `status.sudo == "active"`.
  - Findutils:
    - If AUR install can be run under a configured non-root helper, `status.findutils == "active"`; otherwise a clear error instructs user action (no silent SKIPs) and `status` remains `unset` for findutils.

## References

- Destination directory: `cargo/oxidizr-cli-core/src/packages.rs` (`DEST_DIR = "/usr/bin"`).
- Source selection: `cargo/oxidizr-arch/src/commands/use_cmd.rs` (`resolve_source_bin`, pacman/paru install logic).
- Status reporter: `cargo/oxidizr-arch/src/commands/status.rs`.
- Testing policy: `docs/testing/TESTING_POLICY.md` — harness must not install product-managed artifacts; the CLI must perform all mutations.

## Next steps (planned)

- Implement pacman-driven dispatcher detection in `resolve_source_bin()` and adjust fallback order.
- Improve error messaging and optional delegation for AUR installs under root.
- Make minimal proof script tweaks to avoid `tee` after apply.
- Source binary resolution (Arch):
  - `cargo/oxidizr-arch/src/commands/use_cmd.rs` → `resolve_source_bin(pkg: Package)` builds a candidate list for the unified replacement binary per package and picks the first that exists.
- Applet enumeration:
  - `cargo/oxidizr-cli-core/src/coverage2.rs` → `resolve_applets_for_use()` intersects discovered applets of the replacement with the distro-provided commands (via adapter) when on the live root.
  - `cargo/oxidizr-arch/src/adapters/arch_adapter.rs` → For live root, `pacman -Ql <pkg>` is used to enumerate `/usr/bin/*` files for each package.
- Symlink swap engine:
  - `cargo/switchyard/src/api/apply/executors/ensure_symlink.rs` (per-action logic)
  - `cargo/switchyard/src/fs/swap.rs` → `replace_file_with_symlink_with_override()` performs atomic symlink swap with backup.
- Status reporting:
  - `cargo/oxidizr-arch/src/commands/status.rs` → Considers a package active if any representative applet is a symlink in `"/usr/bin"` (coreutils: `ls`, `cat`, `echo`, `mv`; findutils: `find`, `xargs`; sudo: `sudo`).

## Likely root causes

- __[source-bin mismatch on Arch]__ For coreutils, Arch repo packaging appears to provide the unified dispatcher as `/usr/bin/coreutils` (not `/usr/bin/uutils`, and not `/usr/lib/uutils-coreutils/uutils`). When we link applets to a non-existent source (e.g., `/usr/bin/uutils` or `/usr/lib/uutils-coreutils/uutils`), the target applet becomes a dangling symlink. Evidence:
  - In several runs, `readlink /usr/bin/ls` produced one of the above, and `exists(target)` was false. This explains `status: unset`.
- __[findutils source path variability]__ The AUR package `uutils-findutils-bin` may install the unified binary as `/usr/lib/uutils-findutils/findutils` or `/usr/bin/findutils` depending on the particular PKGBUILD and version (tarball layout).
- __[status heuristic drift]__ The current status logic uses a small fixed representative set (`ls`, `cat`, `echo`, `mv`) for coreutils. If the plan did not cover these for any reason (e.g., discovery anomalies), status would report `unset` even if many other applets were linked. While less likely here (Arch adapter should enumerate these), it is a contributing fragility.
- __[PATH/which noise in proof logs]__ On Arch, `which ls` previously printed `/usr/sbin/ls` but resolved to `/usr/bin/ls` via symlink chain; this is benign but caused confusion in earlier ad-hoc diagnostics. The updated proof script no longer relies on GNU tools and examines the link target path directly.

## Why doesn’t Switchyard prevent this?

Switchyard applies the plan exactly: ensure symlink from `source_bin` → `/usr/bin/<applet>`. If `source_bin` exists at apply-time, the swap succeeds; if not, we can still produce a symlink (to a non-existent target) depending on timing and degraded paths. The engine is not verifying that `source_bin` remains valid after package operations; that verification is the caller’s responsibility. Our CLI should select the correct `source_bin` for the target distro packaging.

## Concrete fixes

- __[Fix source-bin detection on Arch]__
  - For `coreutils`, prefer `/usr/bin/coreutils` first, then fall back to `/usr/bin/uutils`, and only as a last resort `/usr/lib/uutils-coreutils/uutils`.
  - For `findutils`, prefer `/usr/bin/findutils` first, then `/usr/lib/uutils-findutils/findutils`, then `/usr/bin/uutils` as a last resort for dev/unified builds.
  - For `sudo`, keep `/usr/bin/sudo-rs`, fallback `/usr/bin/sudo`.
  - Optionally: dynamically discover the unified path using `pacman -Ql uutils-coreutils | awk` to locate the installed dispatcher binary (more robust than guessing).
  - File: update `resolve_source_bin()` in `cargo/oxidizr-arch/src/commands/use_cmd.rs`.

- __[Harden status]__
  - Option 1 (quick): Expand representative applets or check the actual applets we resolved during `use` and persisted (e.g., store a small manifest, and `status` reads it when available).
  - Option 2 (better): Implement `status --verbose` that enumerates all linked applets under `dest_dir_path()` and, when possible, validates that they point to the current `source_bin` candidate for the package.
  - File: `cargo/oxidizr-arch/src/commands/status.rs`.

- __[Proof script adjustments]__
  - Keep approach of tool-agnostic checks and restore ordering to ensure all package installs complete before `use` begins. The script already attempts this; interleaving in output is likely buffering but the commands are blocking.
  - Consider adding a final hard assertion in the proof: `test -L /usr/bin/ls && [ -x "$(readlink /usr/bin/ls)" ]` (we added a Python fallback already).

## Acceptance criteria

- Running `scripts/arch_dev_proof.sh` prints:
  - `{"event":"use.exec.apply_ok","executed_actions":>0}` for each package.
  - `oxidizr-arch status --json` shows `"active"` for coreutils/findutils/sudo.
  - Proof’s symlink diagnostic reports `/usr/bin/ls` is a symlink to an existing executable.

## Proposed implementation order

1. Update `resolve_source_bin()` candidate ordering to prefer known Arch repo paths.
2. Add an optional dynamic resolver using `pacman -Ql` for `uutils-coreutils`/`uutils-findutils` to locate the installed dispatcher path.
3. Improve `status.rs` to either:
   - check a couple of applet canaries confirmed by the resolver, or
   - support a verbose mode that enumerates linked applets and validates targets.
4. Re-run `scripts/arch_dev_proof.sh` until the status is green consistently.

## References

- Destination directory: `cargo/oxidizr-cli-core/src/packages.rs` (`DEST_DIR = "/usr/bin"`).
- Arch applet enumeration: `cargo/oxidizr-arch/src/adapters/arch_adapter.rs` (uses `pacman -Ql`).
- Applet discovery: `cargo/oxidizr-cli-core/src/coverage2.rs`.
- Source bin selection: `cargo/oxidizr-arch/src/commands/use_cmd.rs` (`resolve_source_bin`).
- Symlink engine:
  - `cargo/switchyard/src/api/apply/executors/ensure_symlink.rs`
  - `cargo/switchyard/src/fs/swap.rs`
- Status reporter: `cargo/oxidizr-arch/src/commands/status.rs`.
