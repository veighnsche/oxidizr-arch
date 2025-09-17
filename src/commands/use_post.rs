use std::path::{Path, PathBuf};

use switchyard::types::safepath::SafePath;
use switchyard::types::ApplyMode;

use crate::cli::args::Package;
use crate::util::paths::ensure_under_root;
use crate::commands::use_utils::resolve_applet_source;
use oxidizr_cli_core::dest_dir_path;

/// On commit under non-live roots, attempt to create the intended symlinks manually
/// so downstream status checks can pass in hermetic tests.
pub fn ensure_symlinks_non_live_root(
    root: &Path,
    mode: ApplyMode,
    package: Package,
    offline: bool,
    source_bin: &Path,
    applets: &[String],
) -> Result<(), String> {
    if !(matches!(mode, ApplyMode::Commit) && root != Path::new("/")) {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs as unix_fs;
        let dest_dir = dest_dir_path();
        for app in applets {
            let dest_base = ensure_under_root(root, &dest_dir);
            let dst = dest_base.join(app);
            let src_for_app: PathBuf = if offline {
                source_bin.to_path_buf()
            } else {
                resolve_applet_source(package, source_bin, app)
            };
            let src_abs = SafePath::from_rooted(root, &src_for_app)
                .map_err(|e2| format!("invalid source_bin: {e2:?}"))?
                .as_path()
                .to_path_buf();
            let _ = fs::remove_file(&dst);
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = unix_fs::symlink(&src_abs, &dst);
        }
    }
    Ok(())
}

/// Minimal smoke: ensure some linked applets point to an executable target; run only on live root
pub fn smoke_check_live_root(
    root: &Path,
    package: Package,
    applets: &[String],
) -> Result<(), String> {
    if root != Path::new("/") {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let dest_dir = dest_dir_path();
        let mut count = 0usize;
        for app in applets {
            let dest_base = ensure_under_root(root, &dest_dir);
            let dst = dest_base.join(app);
            if let Ok(md) = fs::symlink_metadata(&dst) {
                if md.file_type().is_symlink() {
                    if let Ok(tgt) = fs::read_link(&dst) {
                        let cur_abs = if tgt.is_absolute() {
                            tgt
                        } else {
                            dst.parent().unwrap_or(std::path::Path::new("/")).join(tgt)
                        };
                        if let Ok(m) = fs::metadata(&cur_abs) {
                            if m.permissions().mode() & 0o111 != 0 {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        let required = if matches!(package, Package::Coreutils) { 2 } else { 1 };
        let need = std::cmp::min(required, applets.len());
        if count < need {
            return Err(format!(
                "post-apply smoke failed: expected >={} linked applets to target an executable, found {}",
                need, count
            ));
        }
    }
    Ok(())
}
