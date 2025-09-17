use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use switchyard::logging::JsonlSink;
use switchyard::types::safepath::SafePath;
use switchyard::types::{ApplyMode, LinkRequest, PlanInput};
use switchyard::Switchyard;

use crate::adapters::arch::pm_lock_message;
use crate::adapters::arch_adapter::ArchAdapter;
use crate::adapters::preflight::sudo_guard;
use crate::cli::args::Package;
use crate::util::paths::ensure_under_root;
use oxidizr_cli_core::dest_dir_path;
use oxidizr_cli_core::{resolve_applets_for_use, PackageKind};
use serde_json::json;

#[allow(unused_variables)]
pub fn exec(
    api: &Switchyard<JsonlSink, JsonlSink>,
    root: &Path,
    package: Package,
    offline: bool,
    use_local: Option<PathBuf>,
    mode: ApplyMode,
) -> Result<(), String> {
    // Lock check on live root for commit
    let live_root = root == Path::new("/");
    if matches!(mode, ApplyMode::Commit) {
        if let Some(msg) = pm_lock_message(root) {
            return Err(msg);
        }
    }

    // Map packages to Arch replacement and distro package names
    let (rs_pkg, _distro_pkg) = match package {
        Package::Coreutils => ("uutils-coreutils", "coreutils"),
        Package::Findutils => ("uutils-findutils", "findutils"),
        Package::Sudo => ("sudo-rs", "sudo"),
    };

    // Ensure replacement present when committing (ignore when offline=true)
    if matches!(mode, ApplyMode::Commit) && !offline {
        if !live_root {
            eprintln!(
                "[info] skipping pacman/paru install under non-live root: {}",
                root.display()
            );
        } else {
            if !pacman_installed(rs_pkg) {
                // Try pacman first (official), else paru (AUR)
                let mut tried = Vec::new();
                let mut ok = false;
                let mut last_code = 1;
                // pacman -S --noconfirm
                tried.push(format!("pacman -S --noconfirm {}", rs_pkg));
                let mut cmd = Command::new("pacman");
                cmd.args(["-S", "--noconfirm", rs_pkg]);
                cmd.stdin(Stdio::null());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                if let Ok(out) = cmd.output() {
                    last_code = out.status.code().unwrap_or(1);
                    eprintln!(
                        "{}",
                        json!({
                            "event":"pm.install","pm":{"tool":"pacman","args":["-S","--noconfirm",rs_pkg],"package":rs_pkg},
                            "exit_code": last_code,
                            "stderr_tail": String::from_utf8_lossy(&out.stderr).chars().rev().take(400).collect::<String>().chars().rev().collect::<String>()
                        })
                    );
                    ok = out.status.success();
                }
                if !ok {
                    // paru -S --noconfirm
                    tried.push(format!("paru -S --noconfirm {}", rs_pkg));
                    let paru = which::which("paru").ok();
                    if let Some(paru_bin) = paru {
                        let mut cmd = Command::new(paru_bin);
                        cmd.args(["-S", "--noconfirm", rs_pkg]);
                        cmd.stdin(Stdio::null());
                        cmd.stdout(Stdio::piped());
                        cmd.stderr(Stdio::piped());
                        if let Ok(out) = cmd.output() {
                            last_code = out.status.code().unwrap_or(1);
                            eprintln!(
                                "{}",
                                json!({
                                    "event":"pm.install","pm":{"tool":"paru","args":["-S","--noconfirm",rs_pkg],"package":rs_pkg},
                                    "exit_code": last_code,
                                    "stderr_tail": String::from_utf8_lossy(&out.stderr).chars().rev().take(400).collect::<String>().chars().rev().collect::<String>()
                                })
                            );
                            ok = out.status.success();
                        }
                    } else {
                        eprintln!(
                            "[warn] paru not found; cannot install AUR package {} automatically",
                            rs_pkg
                        );
                    }
                }
                if !ok {
                    return Err(format!(
                        "failed to install {} (tried: {}; last_code={})",
                        rs_pkg,
                        tried.join("; "),
                        last_code
                    ));
                }
            }
        }
    } else if matches!(mode, ApplyMode::DryRun) && !offline {
        eprintln!(
            "[dry-run] would run: pacman -S --noconfirm {} (or paru -S)",
            rs_pkg
        );
    }

    // Resolve a plausible multi-call or single-binary source path
    let source_bin = if offline {
        if let Some(p) = use_local.clone() { p } else { return Err("--offline requires --use-local PATH".to_string()); }
    } else {
        resolve_source_bin(package)
    };
    // Preflight: for sudo on commit, require setuid root
    if matches!(mode, ApplyMode::Commit) {
        if matches!(package, Package::Sudo) {
            sudo_guard(root, &source_bin)?;
        }
    }

    // Compute applets via shared core (dynamic discovery + distro intersection on live root)
    let pkg_kind = match package {
        Package::Coreutils => PackageKind::Coreutils,
        Package::Findutils => PackageKind::Findutils,
        Package::Sudo => PackageKind::Sudo,
    };
    let applets = resolve_applets_for_use(&ArchAdapter, root, pkg_kind, &source_bin);

    // Build link plan
    let dest_dir = dest_dir_path();
    let mut links = Vec::new();
    for app in &applets {
        let dest_base = ensure_under_root(root, &dest_dir);
        let dst = dest_base.join(app);
        let s_sp = SafePath::from_rooted(root, &source_bin)
            .map_err(|e| format!("invalid source_bin: {e:?}"))?;
        let d_sp = SafePath::from_rooted(root, &dst).map_err(|e| format!("invalid dest: {e:?}"))?;
        links.push(LinkRequest {
            source: s_sp.clone(),
            target: d_sp,
        });
    }

    let plan = api.plan(PlanInput {
        link: links,
        restore: vec![],
    });
    let _pre = api
        .preflight(&plan)
        .map_err(|e| format!("preflight failed: {e:?}"))?;
    let rep = api
        .apply(&plan, mode)
        .map_err(|e| format!("apply failed: {e:?}"))?;

    if matches!(mode, ApplyMode::DryRun) {
        eprintln!("dry-run: planned {} actions", rep.executed.len());
    } else {
        // Minimal smoke: ensure some symlinks point to source_bin
        #[cfg(unix)]
        {
            use std::fs;
            let mut count = 0usize;
            let src = SafePath::from_rooted(root, &source_bin)
                .map_err(|e| format!("invalid source_bin: {e:?}"))?
                .as_path()
                .to_path_buf();
            for app in &applets {
                let dest_base = ensure_under_root(root, &dest_dir);
                let dst = dest_base.join(app);
                if let Ok(md) = fs::symlink_metadata(&dst) {
                    if md.file_type().is_symlink() {
                        if let Ok(cur) = fs::read_link(&dst) {
                            if cur == src {
                                count += 1;
                            }
                        }
                    }
                }
            }
            let required = if matches!(package, Package::Coreutils) { 2 } else { 1 };
            let need = std::cmp::min(required, applets.len());
            if count < need {
                return Err(format!("post-apply smoke failed: expected >={} links to point to replacement, found {}", need, count));
            }
        }
    }

    Ok(())
}

fn pacman_installed(name: &str) -> bool {
    let st = Command::new("pacman")
        .args(["-Qi", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(st, Ok(s) if s.success())
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
    // Fallback
    match pkg {
        Package::Sudo => PathBuf::from("/usr/bin/sudo"),
        _ => PathBuf::from("/usr/bin/uutils"),
    }
}
