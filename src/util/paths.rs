use std::path::{Path, PathBuf};

pub fn ensure_under_root(root: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        let rel = p.strip_prefix(Path::new("/")).unwrap_or(p);
        root.join(rel)
    } else {
        root.join(p)
    }
}
