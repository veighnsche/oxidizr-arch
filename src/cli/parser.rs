use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// AUR helper selection
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum AurHelperArg {
    /// Auto-detect installed helper (prefer paru, then yay, then trizen, then pamac)
    Auto,
    /// Do not use any AUR helper (pacman only)
    None,
    Paru,
    Yay,
    Trizen,
    Pamac,
}

impl AurHelperArg {
    pub fn as_helper_str(&self) -> &'static str {
        match self {
            AurHelperArg::Auto => "auto",
            AurHelperArg::None => "none",
            AurHelperArg::Paru => "paru",
            AurHelperArg::Yay => "yay",
            AurHelperArg::Trizen => "trizen",
            AurHelperArg::Pamac => "pamac",
        }
    }
}

/// Main CLI structure - backward compatible with original
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "oxidizr-arch style coreutils switching (scaffold)"
)]
pub struct Cli {
    /// Skip confirmation prompts (dangerous; intended for automation/tests)
    /// Accept legacy alias --yes for compatibility with test suites
    #[arg(long, short = 'y', alias = "yes", global = true)]
    pub assume_yes: bool,

    /// Do not run pacman -Sy before actions
    #[arg(long, global = true)]
    pub no_update: bool,

    /// Select all known experiments from the registry
    #[arg(long, short = 'a', global = true, conflicts_with_all = ["experiments", "experiment"])]
    pub all: bool,

    /// Select which experiments to operate on (comma separated or repeatable)
    #[arg(long, value_delimiter = ',', global = true, conflicts_with = "all")]
    pub experiments: Vec<String>,

    /// Backward compatibility: single experiment selection (deprecated)
    #[arg(long, global = true, conflicts_with_all = ["all", "experiments"])]
    pub experiment: Option<String>,

    /// Skip compatibility checks (dangerous)
    #[arg(
        long = "skip-compat-check",
        alias = "skip-compatibility-check",
        global = true
    )]
    pub no_compatibility_check: bool,

    /// AUR helper to use for package operations (auto-detect by default)
    #[arg(long, value_enum, default_value_t = AurHelperArg::Auto, global = true)]
    pub aur_helper: AurHelperArg,

    /// Override package name (Arch/AUR). Defaults depend on experiment.
    #[arg(long, global = true)]
    pub package: Option<String>,

    /// Override bin directory containing replacement binaries
    #[arg(long, global = true)]
    pub bin_dir: Option<std::path::PathBuf>,

    /// Optional unified dispatch binary path (e.g., /usr/bin/coreutils)
    #[arg(long, global = true)]
    pub unified_binary: Option<std::path::PathBuf>,

    /// Dry-run: print planned actions without making changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Wait for pacman database lock to clear, in seconds (polling)
    #[arg(long, alias = "wait_lock", global = true)]
    pub wait_lock: Option<u64>,

    /// Disable progress bars even on TTY (CI-friendly)
    #[arg(long, global = true)]
    pub no_progress: bool,

    /// Run AUR helper as this user (if set). If unset, run as invoking user.
    #[arg(long, global = true)]
    pub aur_user: Option<String>,

    /// Force restore to be best-effort (warn on missing backup instead of error)
    #[arg(long, global = true)]
    pub force_restore_best_effort: bool,

    /// Strict owner verification: abort when target not owned by expected packages
    #[arg(long, global = true)]
    pub strict_ownership: bool,

    /// Force override trust checks for custom --bin_dir/--unified_binary sources
    #[arg(long = "force", global = true)]
    pub force_override_untrusted: bool,

    /// Override state directory for persistence (useful in tests)
    #[arg(long, global = true)]
    pub state_dir: Option<PathBuf>,

    /// Override log directory for state report emission (useful in tests)
    #[arg(long, global = true)]
    pub log_dir: Option<PathBuf>,

    /// Optional user to try sudo -n true smoke test with sudo-rs
    #[arg(long, global = true)]
    pub sudo_smoke_user: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Enable the Rust replacement utilities (install + symlink swap-in)
    Enable,
    /// Disable the Rust replacement utilities (restore + remove)
    Disable,
    /// Remove provider packages after restoring originals
    Remove,
    /// Check distro compatibility for this experiment
    Check,
    /// List computed target paths that would be affected
    ListTargets,
    /// Re-apply links for previously managed experiments (used by pacman hook)
    RelinkManaged,
    /// Install pacman post-transaction hook to relink managed targets
    InstallHook,
}
