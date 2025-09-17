use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::os::unix::fs::PermissionsExt;

use switchyard::logging::JsonlSink;
use switchyard::types::ApplyMode;
use switchyard::Switchyard;

use crate::adapters::arch::pm_lock_message;
use crate::adapters::arch_adapter::ArchAdapter;
use crate::cli::args::{Package, ParityLevel};
use crate::util::selinux::selinux_enabled;
use crate::util::paths::ensure_under_root;
use crate::commands::replace_utils::{
    link_points_to_exec,
    verify_link_points_to,
    resolve_source_bin,
    guess_artifact_path,
    remove_distro_packages,
};
use crate::commands::replace_parity::{enforce_replace_parity, filter_postverify_names};
use oxidizr_cli_core::dest_dir_path;
use oxidizr_cli_core::DistroAdapter;
use oxidizr_cli_core::{coverage_preflight, PackageKind};
use oxidizr_cli_core::packages::{coreutils_critical_set, coreutils_selinux_set, static_fallback_applets};
use serde_json::json;
use switchyard::types::safepath::SafePath;

#[allow(unused_variables)]
pub fn exec(
    api: &Switchyard<JsonlSink, JsonlSink>,
    root: &Path,
    package: Option<Package>,
    all: bool,
    mode: ApplyMode,
    assume_yes: bool,
    parity: ParityLevel,
) -> Result<(), String> {
    let adapter = ArchAdapter;
    let live_root = root == Path::new("/");
    let targets: Vec<Package> = if all {
        vec![Package::Coreutils, Package::Findutils, Package::Sudo]
    } else if let Some(p) = package {
        vec![p]
    } else {
        return Err("specify a package or use --all".to_string());
    };

    if matches!(mode, ApplyMode::Commit) && live_root {
        if let Some(msg) = pm_lock_message(root) {
            return Err(msg);
        }
    }

    // For each target, first activate replacement via `use`, then enforce parity gates
    for p in &targets {
        let (offline, use_local) = if root != Path::new("/") {
            if let Some(path) = guess_artifact_path(root, *p) {
                (true, Some(path))
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };

        // Ensure RS is installed & active using `use` semantics (no parity enforcement here)
        crate::commands::r#use::exec(api, root, *p, offline, use_local.clone(), mode, parity, None)?;

        // Enforce parity gates (Replace semantics) and emit summaries
        let _ready = enforce_replace_parity(&adapter, root, *p, parity, offline, &use_local)?;
    }

    // Snapshot distro-provided names for post-verify (only for coreutils/findutils)
    let mut verify_sets: Vec<(Package, Vec<String>, PathBuf)> = Vec::new();
    for p in &targets {
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
            // Filter names by parity policy for post-verify
            let names = filter_postverify_names(names_raw, root, *p, parity);
            verify_sets.push((*p, names, source_bin));
        }
    }

    // Remove distro packages (commit) or preview (dry-run)
    let (distro_names, _rs_names): (Vec<&str>, Vec<&str>) = targets
        .iter()
        .map(|p| match p {
            Package::Coreutils => ("coreutils", "uutils-coreutils"),
            Package::Findutils => ("findutils", "uutils-findutils"),
            Package::Sudo => ("sudo", "sudo-rs"),
        })
        .unzip();

    remove_distro_packages(root, live_root, mode, &distro_names)?;

    if matches!(mode, ApplyMode::Commit) {
        // Post-verify: for each captured name, ensure it exists and resolves to replacement
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
        // Final friendly summary
        let sel_summary = if selinux_enabled(root) { "enabled" } else { "disabled" };
        println!(
            "[DONE] replace: parity={} (selinux={}) removed={} package(s)",
            format!("{:?}", parity).to_lowercase(),
            sel_summary,
            distro_names.len()
        );
    }

    Ok(())
}
