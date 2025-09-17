# Coreutils Parity Policy (Arch focus; GNU ↔ uutils)

This document specifies the policy and acceptance criteria for switching between GNU coreutils and uutils-coreutils on Arch Linux. It uses RFC‑2119 keywords (MUST, SHOULD, MAY) to define normative requirements for the oxidizr-arch CLI.

## Scope

Covers when the CLI uses uutils applets (via symlink/exec) or replaces GNU coreutils, with special handling for SELinux-only applets (`chcon`, `runcon`) that Arch’s official `uutils-coreutils` does not ship.

## Definitions

- Coverage: the set of applets present in the selected provider (uutils or GNU).
- Critical Set (standard): common, non-SELinux applets required for a functional system (e.g., `ls`, `cp`, `mv`, `rm`, `mkdir`, `ln`, `readlink`, `cat`, `echo`, `date`, `touch`, `chmod`, `chown`, `realpath`, `mktemp`, `paste`, `cut`, `sort`, `uniq`, `tr`, `wc`, `tee`, `head`, `tail`, `env`, `printenv`, `sleep`, `pwd`, `basename`, `dirname`, `test`, `true`, `false`).
- SELinux Set: `chcon`, `runcon`.

## Mode Semantics

- Use mode: prefer uutils where available; do not remove GNU. Mixed tree is OK.
- Replace mode: remove GNU provider for covered applets and rely on uutils; subject to preflight gates.

## Parity Gates (MUST / SHOULD / MAY)

- Replace mode MUST achieve 100% coverage of the Critical Set on the target system.
- Replace mode MUST NOT proceed if SELinux is enabled and the selected uutils build lacks the SELinux Set.
- Use mode MAY proceed with partial coverage; missing applets fall back to GNU.

## SELinux Detection

The CLI MUST detect SELinux status:

- Treat SELinux as enabled if `/sys/fs/selinux` exists AND `getenforce` returns `Enforcing` or `Permissive`.
- Otherwise, treat SELinux as disabled.

When SELinux is disabled, the SELinux Set is OPTIONAL for parity. When enabled, it becomes REQUIRED in Replace mode.

## Provider Detection (Arch specifics)

The CLI MUST detect presence of `uu-chcon`/`uu-runcon` via any of:

- `command -v uu-chcon` / `command -v uu-runcon`, or
- Package file lists (e.g., `pacman -Ql uutils-coreutils | grep -E 'uu-(chcon|runcon)(\\.1\\.gz)?$'`).

If missing in Arch Extra’s `uutils-coreutils`, the CLI SHOULD suggest a SELinux-enabled build (e.g., AUR variant) when SELinux is enabled.

## User Controls

The CLI MUST support a parity threshold flag:

- `--require-parity=standard|strict|none|selinux`
  - standard (default): Critical Set only; SELinux Set required iff SELinux enabled.
  - strict: Critical Set plus all known GNU applets available in the chosen uutils build.
  - selinux: enforce presence of SELinux Set regardless of system SELinux status.
  - none: no parity gate (dangerous; for experts only).

The CLI MAY accept `--allow-missing=<applet1,applet2,...>` to override non-SELinux gaps in Use mode only.

## Safety & Rollback

- Replace mode MUST stage atomic changes and keep a rollback plan (shadow links + revert script).
- Replace mode SHOULD run a smoke test of representative applets (`ls`, `cp`, `mv`, `rm`, `chmod`, `chown`, `readlink`, `realpath`, `mktemp`) before committing.

## Reporting & Telemetry

After any action, the CLI MUST print:

- A one‑line summary including provider and parity status.
- A detailed list of skipped or missing applets, and the parity threshold applied.

Examples:

- Active provider: `uutils`
- Skipped: `chcon`, `runcon` (source_missing in this build; SELinux disabled)
- Or: Aborted: Replace mode blocked — SELinux enabled but `uu-chcon`/`uu-runcon` missing.

`health` MUST report current parity vs threshold and return non‑zero when below threshold.

## Decision Matrix

| SELinux status | `uu-chcon/runcon` present? | Mode    | Policy outcome |
| --- | --- | --- | --- |
| Disabled | No  | Use     | Proceed; log `Skipped: chcon, runcon (selinux_disabled)` |
| Disabled | No  | Replace | Proceed if Critical Set covered; log skipped SELinux applets |
| Enabled  | No  | Use     | Proceed; warn that SELinux applets remain GNU |
| Enabled  | No  | Replace | Block with actionable hint: install SELinux‑enabled uutils build |
| Enabled  | Yes | Replace | Proceed if Critical Set covered; include SELinux Set |

## Example Logs

Use mode on Arch (SELinux disabled):

```
{"event":"use.exec.skip_applet","applet":"chcon","reason":"source_missing","source":"/usr/bin/uu-chcon"}
{"event":"use.exec.skip_applet","applet":"runcon","reason":"source_missing","source":"/usr/bin/uu-runcon"}
Summary: provider=uutils (mixed); skipped=[chcon, runcon]; parity=OK (standard)
```

Replace mode with SELinux enabled but missing applets:

```
Abort: SELinux detected (Permissive). Missing uu-chcon, uu-runcon.
Hint: install SELinux-enabled uutils build and retry with --require-parity=selinux|strict
```

## Rationale (why this works well on Arch)

- Arch’s official `uutils-coreutils` omits `uu-chcon`/`uu-runcon`; treating them as optional when SELinux is off avoids false failures.
- When SELinux is on, requiring those applets prevents silent loss of functionality.
- Parity thresholds keep “replace” safe while allowing “use” to be pragmatic.

## References

- Per‑applet linking and skip logging: `cargo/oxidizr-arch/src/commands/use_cmd.rs`
- Status semantics (executable target check): `cargo/oxidizr-arch/src/commands/status.rs`
- Arch adapter applet enumeration: `cargo/oxidizr-arch/src/adapters/arch_adapter.rs`
- Coverage and parity checks: `oxidizr-cli-core` (coverage discovery) and Switchyard preflight
- Proof scripts: `scripts/arch_dev_proof.sh`, `scripts/arch_dev_shell.sh`
- SELinux feature notes (uutils): <https://github.com/uutils/coreutils/wiki/Supporting-SELinux-in-the-coreutils>
- Arch package file list (uutils-coreutils): <https://archlinux.org/packages/extra/x86_64/uutils-coreutils/files/>
