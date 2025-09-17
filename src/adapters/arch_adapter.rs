use std::path::Path;
use std::process::Command;

use oxidizr_cli_core::{DistroAdapter, PackageKind};

pub struct ArchAdapter;

impl DistroAdapter for ArchAdapter {
    fn enumerate_package_commands(&self, root: &Path, pkg: PackageKind) -> Vec<String> {
        if root != Path::new("/") {
            return Vec::new();
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
