use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;

use crate::cli::args::Package;
use switchyard::types::ApplyMode;
use serde_json::json;
use std::process::{Command, Stdio};

pub fn link_points_to_exec(root: &Path, name: &str) -> bool {
    let link_path = root.join("usr/bin").join(name);
    let md = match std::fs::symlink_metadata(&link_path) { Ok(m) => m, Err(_) => return false };
    if !md.file_type().is_symlink() { return false; }
    let tgt = match std::fs::read_link(&link_path) { Ok(t) => t, Err(_) => return false };
    let abs = if tgt.is_absolute() { tgt } else { link_path.parent().unwrap_or(Path::new("/")).join(tgt) };
    match std::fs::metadata(&abs) { Ok(m) => m.permissions().mode() & 0o111 != 0, Err(_) => false }
}

pub fn remove_distro_packages(
    root: &Path,
    live_root: bool,
    mode: ApplyMode,
    distro_names: &[&str],
) -> Result<(), String> {
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
                    (*name).to_string(),
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
                } else {
                    println!("[OK] replace: removed package {}", name);
                }
            }
        }
    } else {
        for name in distro_names {
            eprintln!("[dry-run] would run: pacman -R --noconfirm {}", name);
            println!("[DRY-RUN] replace: would remove package {}", name);
        }
    }
    Ok(())
}

pub fn verify_link_points_to(dst: &Path, src: &Path) -> bool {
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

pub fn resolve_source_bin(pkg: Package) -> PathBuf {
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

pub fn guess_artifact_path(root: &Path, pkg: Package) -> Option<PathBuf> {
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
