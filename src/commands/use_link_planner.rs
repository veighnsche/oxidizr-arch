use std::path::{Path, PathBuf};

use switchyard::types::safepath::SafePath;
use switchyard::types::LinkRequest;

use crate::cli::args::Package;
use crate::util::paths::ensure_under_root;
use crate::commands::use_utils::resolve_applet_source;
use oxidizr_cli_core::dest_dir_path;

/// Build the link requests for a set of applets. Returns (links, skipped_applets).
pub fn plan_links(
    root: &Path,
    package: Package,
    offline: bool,
    source_bin: &Path,
    applets: &[String],
) -> Result<(Vec<LinkRequest>, Vec<String>), String> {
    let dest_dir = dest_dir_path();
    let mut links: Vec<LinkRequest> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for app in applets {
        let dest_base = ensure_under_root(root, &dest_dir);
        let dst = dest_base.join(app);
        let src_for_app: PathBuf = if offline {
            source_bin.to_path_buf()
        } else {
            resolve_applet_source(package, source_bin, app)
        };

        // Avoid creating dangling symlinks: require that source exists and is executable
        if let Ok(md) = std::fs::metadata(&src_for_app) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if md.permissions().mode() & 0o111 == 0 {
                    skipped.push(app.clone());
                    continue;
                }
            }
        } else {
            skipped.push(app.clone());
            continue;
        }

        let s_sp = SafePath::from_rooted(root, &src_for_app)
            .map_err(|e| format!("invalid source_bin: {e:?}"))?;
        let d_sp = SafePath::from_rooted(root, &dst).map_err(|e| format!("invalid dest: {e:?}"))?;
        links.push(LinkRequest {
            source: s_sp,
            target: d_sp,
        });
    }

    Ok((links, skipped))
}
