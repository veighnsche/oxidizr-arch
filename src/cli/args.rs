use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Package {
    Coreutils,
    Findutils,
    Sudo,
}

#[derive(Debug, Parser)]
#[command(
    name = "oxidizr-arch",
    version,
    about = "Arch Linux CLI to swap GNU coreutils/findutils with uutils and sudo with sudo-rs using Switchyard"
)]
pub struct Cli {
    /// Root of the filesystem tree to operate on (default "/")
    #[arg(long, global = true, default_value = "/")]
    pub root: PathBuf,

    /// Commit changes to disk (default is dry-run)
    #[arg(long, global = true, default_value_t = false)]
    pub commit: bool,

    /// Assume "yes" to any confirmation prompts (non-interactive by default in non-TTY)
    #[arg(long, global = true, default_value_t = false)]
    pub assume_yes: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Use a replacement for a package (install if needed + safe swap)
    Use {
        /// Which package to use
        #[arg(value_enum)]
        package: Package,
        /// Offline mode: use a local artifact instead of fetching
        #[arg(long, default_value_t = false)]
        offline: bool,
        /// Local artifact path when --offline (still validated)
        #[arg(long, value_name = "PATH")]
        use_local: Option<PathBuf>,
    },
    /// Restore GNU/stock tools for a package (or all)
    Restore {
        /// Package to restore; when omitted, restores all known packages
        #[arg(value_enum)]
        package: Option<Package>,
        /// Restore all known packages
        #[arg(long, conflicts_with = "package")]
        all: bool,
        /// Keep RS packages installed but de-preferred
        #[arg(long, default_value_t = false)]
        keep_replacements: bool,
    },
    /// Replace distro packages with Rust replacements (ensure install + safe swap + remove GNU)
    Replace {
        /// Package to replace; when omitted, targets all known packages
        #[arg(value_enum)]
        package: Option<Package>,
        /// Target all known packages
        #[arg(long, conflicts_with = "package")]
        all: bool,
    },
    /// Report current active state
    Status {
        /// Output machine-readable JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Run diagnostics and environment checks
    Doctor {
        /// Output machine-readable JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum, default_value_t = Shell::Bash)]
        shell: Shell,
    },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}
