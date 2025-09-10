use crate::{Error, Result};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::path::Path;

const LOCK_PATH: &str = "/run/lock/oxidizr-arch.lock";

pub struct ProcessLock {
    file: File,
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        // Keep the lock file persistent to avoid TOCTOU on creation; do not delete.
    }
}

pub fn acquire() -> Result<ProcessLock> {
    if let Some(parent) = Path::new(LOCK_PATH).parent() {
        fs::create_dir_all(parent).ok();
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOCK_PATH)?;
    if let Err(_) = file.try_lock_exclusive() {
        return Err(Error::ExecutionFailed(
            "another instance of oxidizr-arch is running (lock held)".into(),
        ));
    }
    Ok(ProcessLock { file })
}
