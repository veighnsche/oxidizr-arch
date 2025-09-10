use crate::{Error, Result};
use std::path::Path;

pub fn ensure_mount_rw_exec(path: &Path) -> Result<()> {
    match switchyard::preflight::ensure_mount_rw_exec(path) {
        Ok(()) => Ok(()),
        Err(msg) => Err(Error::FilesystemUnsuitable(msg)),
    }
}

pub fn check_immutable(path: &Path) -> Result<()> {
    match switchyard::preflight::check_immutable(path) {
        Ok(()) => Ok(()),
        Err(msg) => Err(Error::FilesystemUnsuitable(msg)),
    }
}

pub fn check_source_trust(source: &Path, force: bool) -> Result<()> {
    match switchyard::preflight::check_source_trust(source, force) {
        Ok(()) => Ok(()),
        Err(msg) => {
            if force {
                tracing::warn!("source_trust override: {}", msg);
                Ok(())
            } else {
                Err(Error::ExecutionFailed(msg))
            }
        }
    }
}
