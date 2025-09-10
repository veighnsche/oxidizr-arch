use thiserror::Error;

/// Core error type for oxidizr-arch operations
#[derive(Debug, Error)]
pub enum Error {
    /// CLI usage errors (conflicting flags, missing selections, etc.)
    #[error("CLI misuse: {0}")]
    CliMisuse(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid implementation: {0}")]
    InvalidImplementation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported distribution or release: {0}")]
    Incompatible(String),

    // New typed errors for enumerated exit codes / behavior
    /// No applets discovered to link after ensuring provider installation
    #[error("nothing to link: {0}")]
    NothingToLink(String),

    /// Restore failed because a backup file is missing (unless forced best-effort)
    #[error("restore backup missing for: {0}")]
    RestoreBackupMissing(String),

    /// Repository gating failed for a package with details
    #[error("repo gate failed for '{package}': {details}")]
    RepoGateFailed { package: String, details: String },

    /// Pacman database lock timeout reached
    #[error("pacman DB lock timeout at /var/lib/pacman/db.lck")] 
    PacmanLockTimeout,

    /// Operation requires root privileges
    #[error("This command must be run as root")]
    RootRequired,

    /// Filesystem unsuitable (e.g., ro mount or noexec)
    #[error("filesystem unsuitable: {0}")]
    FilesystemUnsuitable(String),

    /// Pacman hook installation failed
    #[error("hook install error: {0}")]
    HookInstallError(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Map errors to stable process exit codes.
    ///
    /// Codes (subject to future refinement, prepared per TODO):
    ///  1  - General failure (default)
    ///  2  - CLI misuse
    /// 10  - Incompatible distro
    /// 20  - Nothing to link
    /// 30  - Restore backup missing
    /// 40  - Repo/AUR gating failure
    /// 50  - Pacman DB lock timeout
    /// 70  - Root required
    /// 80  - Filesystem immutable/noexec/unsuitable
    /// 90  - Hook install error
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::CliMisuse(_) => 2,
            Error::Incompatible(_) => 10,
            Error::NothingToLink(_) => 20,
            Error::RestoreBackupMissing(_) => 30,
            Error::RepoGateFailed { .. } => 40,
            Error::PacmanLockTimeout => 50,
            Error::RootRequired => 70,
            Error::FilesystemUnsuitable(_) => 80,
            Error::HookInstallError(_) => 90,
            // Fallback for all other errors
            _ => 1,
        }
    }
}
