use std::path::{Path, PathBuf};

use crate::error::Result;
use which::which;

use crate::system::fs_checks;

impl super::Worker {
    /// Find a binary in PATH
    pub fn which(&self, name: &str) -> Result<Option<PathBuf>> {
        Ok(which(name).ok())
    }

    /// Replace file with symlink (delegated to symlink module)
    pub fn replace_file_with_symlink(&self, source: &Path, target: &Path) -> Result<()> {
        // Filesystem preflights
        fs_checks::ensure_mount_rw_exec(Path::new("/usr"))?;
        fs_checks::ensure_mount_rw_exec(target)?;
        fs_checks::check_immutable(target)?;
        // Trust/source checks
        fs_checks::check_source_trust(source, self.force_override_untrusted)?;
        // Ownership policy (warn or abort depending on --strict-ownership)
        self.verify_owner_for_target(target)?;
        crate::symlink::replace_file_with_symlink(source, target, self.dry_run)
    }

    /// Restore file from backup (delegated to symlink module)
    pub fn restore_file(&self, target: &Path) -> Result<()> {
        fs_checks::ensure_mount_rw_exec(Path::new("/usr"))?;
        fs_checks::ensure_mount_rw_exec(target)?;
        fs_checks::check_immutable(target)?;
        crate::symlink::restore_file(target, self.dry_run, self.force_restore_best_effort)
    }
}
