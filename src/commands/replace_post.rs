use std::path::{Path, PathBuf};

use crate::cli::args::{Package, ParityLevel};
use crate::commands::replace_parity::filter_postverify_names;
use crate::commands::replace_utils::{guess_artifact_path, resolve_source_bin, verify_link_points_to};
use crate::util::paths::ensure_under_root;
use oxidizr_cli_core::{DistroAdapter, PackageKind};
use oxidizr_cli_core::dest_dir_path;
use switchyard::types::safepath::SafePath;

/// Build the post-verify set of (package, applet_names, source_bin) according to parity policy.
pub fn collect_verify_sets<A: DistroAdapter>(
    adapter: &A,
    root: &Path,
    targets: &[Package],
    parity: ParityLevel,
) -> Vec<(Package, Vec<String>, PathBuf)> {
    let mut out = Vec::new();
    for p in targets {
        if !matches!(p, Package::Sudo) {
            let kind = if matches!(p, Package::Coreutils) {
                PackageKind::Coreutils
            } else {
                PackageKind::Findutils
            };
            let names_raw = adapter.enumerate_package_commands(root, kind);
            let source_bin = if root != Path::new("/") {
                guess_artifact_path(root, *p).unwrap_or_else(|| resolve_source_bin(*p))
            } else {
                resolve_source_bin(*p)
            };
            let names = filter_postverify_names(names_raw, root, *p, parity);
            out.push((*p, names, source_bin));
        }
    }
    out
}

/// Verify that all dest links point to replacement source bin, with friendly output.
pub fn post_verify_links(
    root: &Path,
    verify_sets: Vec<(Package, Vec<String>, PathBuf)>,
) -> Result<(), String> {
    let dest_dir = dest_dir_path();
    for (pkg, names, src) in verify_sets {
        if names.is_empty() {
            continue;
        }
        let src_sp = SafePath::from_rooted(root, &src)
            .map_err(|e| format!("invalid source_bin: {e:?}"))?;
        let src_path = src_sp.as_path().to_path_buf();
        for app in names {
            let dst = ensure_under_root(root, &dest_dir).join(&app);
            let ok = verify_link_points_to(&dst, &src_path);
            if !ok {
                return Err(format!(
                    "post-verify failed for {:?}: {} does not point to replacement",
                    pkg,
                    dst.display()
                ));
            }
        }
        println!("[OK] replace {:?}: post-verify links look good", pkg);
    }
    Ok(())
}
