use std::path::Path;
use std::process::Command;

use oxidizr_cli_core::dest_dir_path;
use oxidizr_cli_core::{static_fallback_applets, DistroAdapter, PackageKind};

pub struct ArchAdapter;

impl DistroAdapter for ArchAdapter {
    fn enumerate_package_commands(&self, root: &Path, pkg: PackageKind) -> Vec<String> {
        if root != Path::new("/") {
            // Hermetic fallback for tests: enumerate all names under <root>/usr/bin
            let dest = dest_dir_path();
            let base = root.join(dest.strip_prefix("/").unwrap_or(&dest));
            let mut names = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&base) {
                for ent in rd.flatten() {
                    if let Some(name) = ent.file_name().to_str() {
                        names.push(name.to_string());
                    }
                }
            }
            // Filter by package allowlist to emulate per-package enumeration
            let allow = static_fallback_applets(pkg);
            let allow_set: std::collections::HashSet<&str> =
                allow.iter().map(|s| s.as_str()).collect();
            names.retain(|n| allow_set.contains(n.as_str()));
            names.sort();
            names.dedup();
            return names;
        }
        let name = match pkg {
            PackageKind::Coreutils => "coreutils",
            PackageKind::Findutils => "findutils",
            PackageKind::Sudo => "sudo",
        };
        let out = match Command::new("pacman").args(["-Ql", name]).output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };
        if !out.status.success() {
            return Vec::new();
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let mut names = Vec::new();
        for line in s.lines() {
            // Expected format: "pkg /path"
            let mut parts = line.split_whitespace();
            let _pkg = parts.next();
            if let Some(path) = parts.next() {
                if let Some(n) = path
                    .strip_prefix("/usr/bin/")
                    .or_else(|| path.strip_prefix("/bin/"))
                {
                    if !n.is_empty() && !n.ends_with('/') {
                        names.push(n.to_string());
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }
}
