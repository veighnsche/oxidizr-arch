use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use switchyard::logging::JsonlSink;
use switchyard::types::ApplyMode;
use switchyard::Switchyard;

use crate::adapters::arch::pm_lock_message;
use crate::adapters::arch_adapter::ArchAdapter;
use crate::cli::args::Package;
use crate::util::paths::ensure_under_root;
use oxidizr_cli_core::dest_dir_path;
use oxidizr_cli_core::DistroAdapter;
use oxidizr_cli_core::{coverage_preflight, PackageKind};
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

    // For each target, run coverage preflight first, then invoke `use`
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

        // Preflight coverage against current distro set (for coreutils/findutils)
        let kind = match p {
            Package::Coreutils => Some(PackageKind::Coreutils),
            Package::Findutils => Some(PackageKind::Findutils),
            Package::Sudo => None,
        };
        if let Some(k) = kind {
            let source_bin = if offline {
                use_local.clone().unwrap()
            } else {
                resolve_source_bin(*p)
            };
            if let Err(missing) = coverage_preflight(&adapter, root, k, &source_bin) {
                return Err(format!(
                    "coverage preflight failed for {:?}: missing: {}",
                    p,
                    missing.join(", ")
                ));
            }
        }

        // Ensure RS is installed & active using `use` semantics
        crate::commands::r#use::exec(api, root, *p, offline, use_local.clone(), mode)?;
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
            let names = adapter.enumerate_package_commands(root, kind);
            let source_bin = if root != Path::new("/") {
                guess_artifact_path(root, *p).unwrap_or_else(|| resolve_source_bin(*p))
            } else {
                resolve_source_bin(*p)
            };
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

    if matches!(mode, ApplyMode::Commit) {
        if !live_root {
            eprintln!(
                "[info] skipping pacman removals under non-live root: {}",
                root.display()
            );
        } else {
            for name in distro_names {
                let mut cmd = Command::new("pacman");
                let args = vec![
                    "-R".to_string(),
                    "--noconfirm".to_string(),
                    name.to_string(),
                ];
                let args_view = args.clone();
                cmd.args(&args);
                cmd.stdin(Stdio::null());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                let out = cmd
                    .output()
                    .map_err(|e| format!("failed to spawn pacman: {e}"))?;
                let code = out.status.code().unwrap_or(1);
                let stderr_tail = String::from_utf8_lossy(&out.stderr);
                eprintln!(
                    "{}",
                    json!({
                        "event":"pm.remove","pm":{"tool":"pacman","args": args_view, "package": name},
                        "exit_code": code,
                        "stderr_tail": stderr_tail.chars().rev().take(400).collect::<String>().chars().rev().collect::<String>()
                    })
                );
                if code != 0 {
                    return Err(format!("pacman -R {} failed with exit code {}", name, code));
                }
            }
        }
    } else {
        for name in distro_names {
            eprintln!("[dry-run] would run: pacman -R --noconfirm {}", name);
        }
    }

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
        }
    }

    Ok(())
}

fn verify_link_points_to(dst: &Path, src: &Path) -> bool {
    use std::fs;
    if let Ok(md) = fs::symlink_metadata(dst) {
        if md.file_type().is_symlink() {
            if let Ok(cur) = fs::read_link(dst) {
                return cur == src;
            }
        }
    }
    false
}

fn resolve_source_bin(pkg: Package) -> PathBuf {
    let candidates: &[&str] = match pkg {
        Package::Coreutils => &["/usr/bin/uutils", "/usr/lib/uutils-coreutils/uutils"],
        Package::Findutils => &["/usr/bin/uutils"],
        Package::Sudo => &["/usr/bin/sudo-rs", "/usr/bin/sudo"],
    };
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    match pkg {
        Package::Sudo => PathBuf::from("/usr/bin/sudo"),
        _ => PathBuf::from("/usr/bin/uutils"),
    }
}

fn guess_artifact_path(root: &Path, pkg: Package) -> Option<PathBuf> {
    let rel = match pkg {
        Package::Coreutils => "/opt/uutils/uutils",
        Package::Findutils => "/opt/uutils-findutils/uutils-findutils",
        Package::Sudo => "/opt/sudo-rs/sudo-rs",
    };
    let trimmed = rel.trim_start_matches('/');
    let abs = root.join(trimmed);
    if abs.exists() {
        Some(abs)
    } else {
        None
    }
}
