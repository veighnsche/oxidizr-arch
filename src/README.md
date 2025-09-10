# oxidizr-arch — Streamlined Implementation

A clean rewrite of the oxidizr-arch package management and symlink switching system, maintaining full CLI backward compatibility while improving internal structure.

## Architecture

```
src/
├── cli/                  # Command-line interface
│   ├── parser.rs        # Clap CLI definitions (backward compatible)
│   └── handler.rs       # Command execution logic
├── experiments/         # Per-experiment logic
│   ├── coreutils.rs    # uutils-coreutils experiment
│   ├── findutils.rs    # uutils-findutils-bin experiment
│   └── sudors.rs       # sudo-rs experiment
├── system/             # System operations
│   └── worker.rs       # Pacman/AUR, filesystem ops, dry-run support
├── symlink/            # Symlink management
│   └── ops.rs          # Atomic backup/restore operations
├── checks/             # Compatibility validation
│   └── compat.rs       # Distro compatibility checks
├── logging/            # Audit and provenance
│   └── provenance.rs   # Structured command logging
├── error.rs            # Error types
├── lib.rs              # Library entry point
└── main.rs             # Binary entry point
```

## Key Improvements

1. **Cleaner Module Organization**: Flattened structure with clear single responsibilities
2. **Unified Worker**: All system operations in one place with consistent error handling
3. **Explicit Package Policy**: 
   - `uutils-coreutils`: Official repos only
   - `uutils-findutils-bin`: AUR fallback allowed
   - `sudo-rs`: Official repos only
4. **Structured Provenance**: JSONL logging of all commands and decisions
5. **Safety First**: No wildcards, atomic operations, path validation

## Supported Distros

- Arch Linux
- EndeavourOS  
- CachyOS
- Manjaro

Other distros require `--skip-compatibility-check` flag.

## CLI Compatibility

All original flags and commands are preserved:

```bash
# Enable experiments
oxidizr-arch enable --experiments=coreutils,findutils

# Disable (restore only)
oxidizr-arch disable --experiments=sudo-rs

# With prompts skipped
oxidizr-arch enable --all -y

# Dry run mode
oxidizr-arch enable --dry-run

# Skip compatibility check
oxidizr-arch enable --skip-compatibility-check
```

## Building

```bash
cargo build --release
```

## Testing

Tests use temporary directories to avoid permission issues:

```bash
cargo test
```

## Audit Logs

Provenance logs are written to:
- System: `/var/log/oxidizr-arch-audit.log`
- User fallback: `~/.oxidizr-arch-audit.log`

Format: JSONL with timestamp, component, event, decision, inputs, outputs, exit_code
