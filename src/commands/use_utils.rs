use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cli::args::Package;

/// Query pacman filelist for a specific per-applet binary provided by the replacement package.
pub fn pacman_query_applet(pkg: Package, applet: &str) -> Option<PathBuf> {
    let pkg_name = match pkg {
        Package::Coreutils => "uutils-coreutils",
        // Arch/AUR ships findutils replacement as uutils-findutils-bin
        Package::Findutils => "uutils-findutils-bin",
        Package::Sudo => return None,
    };
    let out = Command::new("pacman")
        .args(["-Ql", pkg_name])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let needle = format!("/uu-{}", applet);
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if let Some(path) = line.split_whitespace().nth(1) {
            if path.ends_with(&needle) {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Resolve the source binary for a given applet: prefer per-applet 'uu-<applet>' when present;
/// otherwise fall back to the base dispatcher.
pub fn resolve_applet_source(pkg: Package, base: &Path, applet: &str) -> PathBuf {
    if let Some(p) = pacman_query_applet(pkg, applet) {
        return p;
    }
    let candidates: &[&str] = match pkg {
        Package::Coreutils => &[
            "/usr/bin/uu-", // will be appended
            "/usr/lib/uutils-coreutils/uu-",
        ],
        Package::Findutils => &[
            "/usr/bin/uu-",
            "/usr/lib/uutils-findutils/uu-",
        ],
        Package::Sudo => &[],
    };
    for prefix in candidates {
        let p = PathBuf::from(format!("{}{}", prefix, applet));
        if p.exists() {
            return p;
        }
    }
    base.to_path_buf()
}

/// Query pacman file list for the dispatcher location of replacement packages.
pub fn pacman_query_dispatcher(pkg: Package) -> Option<PathBuf> {
    let pkg_name = match pkg {
        Package::Coreutils => "uutils-coreutils",
        Package::Findutils => "uutils-findutils-bin",
        Package::Sudo => "sudo-rs",
    };
    let out = Command::new("pacman")
        .args(["-Ql", pkg_name])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let suffixes: &[&str] = match pkg {
        Package::Coreutils => &["/uutils", "/coreutils"],
        Package::Findutils => &["/findutils", "/uutils"],
        Package::Sudo => &["/sudo-rs", "/sudo"],
    };
    for line in stdout.lines() {
        if let Some(path) = line.split_whitespace().nth(1) {
            for suf in suffixes {
                if path.ends_with(suf) {
                    let p = PathBuf::from(path);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

/// Resolve a plausible multi-call or single-binary source path (base) for a package.
pub fn resolve_source_bin(pkg: Package) -> PathBuf {
    if let Some(p) = pacman_query_dispatcher(pkg) {
        return p;
    }
    let candidates: &[&str] = match pkg {
        Package::Coreutils => &[
            "/usr/bin/coreutils",
            "/usr/bin/uutils",
            "/usr/lib/uutils-coreutils/uutils",
        ],
        Package::Findutils => &[
            "/usr/bin/findutils",
            "/usr/lib/uutils-findutils/findutils",
            "/usr/bin/uutils",
        ],
        Package::Sudo => &[
            "/usr/bin/sudo-rs",
            "/usr/bin/sudo",
        ],
    };
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    match pkg {
        Package::Coreutils => PathBuf::from("/usr/bin/coreutils"),
        Package::Findutils => PathBuf::from("/usr/bin/findutils"),
        Package::Sudo => PathBuf::from("/usr/bin/sudo"),
    }
}

/// Check whether a pacman package is installed.
pub fn pacman_installed(name: &str) -> bool {
    let st = Command::new("pacman")
        .args(["-Qi", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(st, Ok(s) if s.success())
}
