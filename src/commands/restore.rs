use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use switchyard::logging::JsonlSink;
use switchyard::types::safepath::SafePath;
use switchyard::types::{ApplyMode, PlanInput, RestoreRequest};
use switchyard::Switchyard;

use crate::adapters::arch::pm_lock_message;
use crate::adapters::arch_adapter::ArchAdapter;
use crate::cli::args::Package;
use crate::util::paths::ensure_under_root;
use oxidizr_cli_core::dest_dir_path;
use oxidizr_cli_core::{static_fallback_applets, PackageKind, DistroAdapter};
use serde_json::json;

#[allow(unused_variables)]
pub fn exec(
    api: &Switchyard<JsonlSink, JsonlSink>,
    root: &Path,
    package: Option<Package>,
    all: bool,
    keep_replacements: bool,
    mode: ApplyMode,
    assume_yes: bool,
) -> Result<(), String> {
    let adapter = ArchAdapter;
    let live_root = root == Path::new("/");
    if matches!(mode, ApplyMode::Commit) && live_root {
        if let Some(msg) = pm_lock_message(root) {
            return Err(msg);
        }
    }

    let packages: Vec<Package> = if all {
        vec![Package::Coreutils, Package::Findutils, Package::Sudo]
    } else if let Some(p) = package {
        vec![p]
    } else {
        vec![Package::Coreutils, Package::Findutils, Package::Sudo]
    };

    // Pre: ensure distro packages are installed when committing
    if matches!(mode, ApplyMode::Commit) {
        if !live_root {
            eprintln!(
                "[info] skipping pacman installs under non-live root: {}",
                root.display()
            );
        } else {
            for p in &packages {
                let name = distro_pkg_name(*p);
                let mut cmd = Command::new("pacman");
                let args = vec![
                    "-S".to_string(),
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
                        "event":"pm.install","pm":{"tool":"pacman","args": args_view, "package": name},
                        "exit_code": code,
                        "stderr_tail": stderr_tail.chars().rev().take(400).collect::<String>().chars().rev().collect::<String>()
                    })
                );
                if code != 0 {
                    return Err(format!("pacman -S {} failed with exit code {}", name, code));
                }
            }
        }
    } else {
        for p in &packages {
            eprintln!(
                "[dry-run] would run: pacman -S --noconfirm {}",
                distro_pkg_name(*p)
            );
        }
    }

    // Compute full set of applets to restore
    let mut applets = Vec::new();
    for p in &packages {
        let kind = match p {
            Package::Coreutils => PackageKind::Coreutils,
            Package::Findutils => PackageKind::Findutils,
            Package::Sudo => PackageKind::Sudo,
        };
        let mut set = adapter.enumerate_package_commands(root, kind);
        if set.is_empty() {
            set = static_fallback_applets(kind);
        }
        applets.extend(set);
    }

    // Build restore plan
    let dest_dir = dest_dir_path();
    let mut restores = Vec::new();
    for app in &applets {
        let dest_base = ensure_under_root(root, &dest_dir);
        let dst = dest_base.join(app);
        let sp = SafePath::from_rooted(root, &dst).map_err(|e| format!("invalid target: {e:?}"))?;
        restores.push(RestoreRequest { target: sp });
    }

    let plan = api.plan(PlanInput {
        link: vec![],
        restore: restores,
    });
    let _pre = api
        .preflight(&plan)
        .map_err(|e| format!("preflight failed: {e:?}"))?;
    let _rep = api
        .apply(&plan, mode)
        .map_err(|e| format!("apply failed: {e:?}"))?;

    // Post: optionally purge RS packages unless --keep-replacements
    if matches!(mode, ApplyMode::Commit) {
        if !keep_replacements && live_root {
            for p in &packages {
                let rs_name = replacement_pkg_name(*p);
                if pacman_installed(rs_name) {
                    let mut cmd = Command::new("pacman");
                    let args = vec![
                        "-R".to_string(),
                        "--noconfirm".to_string(),
                        rs_name.to_string(),
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
                            "event":"pm.remove","pm":{"tool":"pacman","args": args_view, "package": rs_name},
                            "exit_code": code,
                            "stderr_tail": stderr_tail.chars().rev().take(400).collect::<String>().chars().rev().collect::<String>()
                        })
                    );
                    if code != 0 {
                        return Err(format!(
                            "pacman -R {} failed with exit code {}",
                            rs_name, code
                        ));
                    }
                }
            }
        }
    } else {
        if !keep_replacements {
            for p in &packages {
                eprintln!(
                    "[dry-run] would run: pacman -R --noconfirm {}",
                    replacement_pkg_name(*p)
                );
            }
        }
    }

    Ok(())
}

fn distro_pkg_name(pkg: Package) -> &'static str {
    match pkg {
        Package::Coreutils => "coreutils",
        Package::Findutils => "findutils",
        Package::Sudo => "sudo",
    }
}

fn replacement_pkg_name(pkg: Package) -> &'static str {
    match pkg {
        Package::Coreutils => "uutils-coreutils",
        Package::Findutils => "uutils-findutils",
        Package::Sudo => "sudo-rs",
    }
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
