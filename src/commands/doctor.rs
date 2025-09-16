use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
pub struct DoctorReport {
    distro_id: String,
    distro_version: Option<String>,
    locks_present: bool,
    locks: Vec<String>,
    paths_ok: bool,
    tips: Vec<String>,
}

fn detect_distro(root: &Path) -> (String, Option<String>) {
    let osr = root.join("etc/os-release");
    if let Ok(s) = fs::read_to_string(osr) {
        let mut map = BTreeMap::new();
        for line in s.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim_matches('"');
                map.insert(k.to_string(), v.to_string());
            }
        }
        let id = map
            .get("ID")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let ver = map.get("VERSION_ID").cloned();
        return (id, ver);
    }
    ("unknown".to_string(), None)
}

fn check_locks(root: &Path) -> (bool, Vec<String>) {
    use fs2::FileExt;
    use std::fs::OpenOptions;
    // pacman DB lock
    let locks = ["/var/lib/pacman/db.lck"];
    let mut held = vec![];
    for l in locks {
        let p = root.join(l.trim_start_matches('/'));
        if !p.exists() { continue; }
        if let Ok(f) = OpenOptions::new().read(true).write(true).open(&p) {
            if f.try_lock_exclusive().is_err() {
                held.push(l.to_string());
            } else {
                let _ = f.unlock();
            }
        }
    }
    (!held.is_empty(), held)
}

fn check_paths(root: &Path) -> bool {
    root.join("usr").is_dir() && root.join("usr/bin").is_dir()
}

pub fn exec(root: &Path, json: bool) -> Result<(), String> {
    let (distro_id, distro_version) = detect_distro(root);
    let (locks_present, locks) = check_locks(root);
    let paths_ok = check_paths(root);
    let mut tips = vec![];
    if locks_present {
        tips.push("Package manager busy (pacman lock detected); retry after current operation finishes.".to_string());
    }
    if !paths_ok {
        tips.push(
            "Missing expected directories under --root (usr/bin); ensure target root is correct.".to_string(),
        );
    }

    if json {
        let rep = DoctorReport { distro_id, distro_version, locks_present, locks, paths_ok, tips };
        println!("{}", serde_json::to_string(&rep).map_err(|e| e.to_string())?);
    } else {
        println!(
            "Detected distro: {} {}",
            distro_id,
            distro_version.clone().unwrap_or_default()
        );
        if locks_present {
            println!("Locks present: yes");
            for l in &locks { println!("  - {}", l); }
        } else {
            println!("Locks present: no");
        }
        println!(
            "Paths ok (usr/bin): {}",
            if paths_ok { "yes" } else { "no" }
        );
        if !tips.is_empty() {
            println!("Tips:");
            for t in &tips { println!("  - {}", t); }
        }
    }
    Ok(())
}
