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
