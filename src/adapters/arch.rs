use fs2::FileExt;
use std::fs::OpenOptions;
use std::path::Path;

pub fn pm_lock_message(root: &Path) -> Option<String> {
    // pacman DB lock
    let lock_path = "/var/lib/pacman/db.lck";
    let p = if root == Path::new("/") {
        Path::new(lock_path).to_path_buf()
    } else {
        root.join(lock_path.trim_start_matches('/'))
    };
    if !p.exists() {
        return None;
    }
    if let Ok(f) = OpenOptions::new().read(true).write(true).open(&p) {
        match f.try_lock_exclusive() {
            Ok(_) => {
                let _ = f.unlock();
                None
            }
            Err(_) => Some("Package manager busy (pacman db.lck detected); retry after current operation finishes.".to_string()),
        }
    } else {
        // If we cannot open the file, be conservative and warn
        Some("Package manager may be busy (pacman db.lck present)".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    fn mk_lock_path(root: &Path) -> std::path::PathBuf {
        let rel = "var/lib/pacman";
        let p = root.join(rel);
        let _ = fs::create_dir_all(&p);
        p.join("db.lck")
    }

    #[test]
    fn test_pacman_lock_guard_when_db_lck_present_and_locked() {
        let t = tempfile::tempdir().unwrap();
        let lock = mk_lock_path(t.path());
        let f = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        // Hold an exclusive lock to simulate pacman running
        f.lock_exclusive().unwrap();
        let msg = pm_lock_message(t.path());
        // Release lock after check
        let _ = f.unlock();
        assert!(msg.is_some(), "expected lock message when db.lck is held");
    }

    #[test]
    fn test_pacman_lock_guard_when_db_lck_present_but_unlocked() {
        let t = tempfile::tempdir().unwrap();
        let lock = mk_lock_path(t.path());
        let _f = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        // No lock taken; pm_lock_message should be able to lock and thus return None
        let msg = pm_lock_message(t.path());
        assert!(
            msg.is_none(),
            "no message when db.lck exists but is not locked"
        );
    }
}
