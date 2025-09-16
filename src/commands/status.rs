use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
struct StatusJson<'a> {
    coreutils: &'a str,
    findutils: &'a str,
    sudo: &'a str,
}

pub fn exec(root: &Path, json: bool) -> Result<(), String> {
    let ls = root.join("usr/bin/ls");
    let find = root.join("usr/bin/find");
    let sudo = root.join("usr/bin/sudo");
    let coreutils_active = ls
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let findutils_active = find
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let sudo_active = sudo
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if json {
        let payload = StatusJson {
            coreutils: if coreutils_active { "active" } else { "unset" },
            findutils: if findutils_active { "active" } else { "unset" },
            sudo: if sudo_active { "active" } else { "unset" },
        };
        println!(
            "{}",
            serde_json::to_string(&payload).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "coreutils: {}",
            if coreutils_active { "active" } else { "unset" }
        );
        println!(
            "findutils: {}",
            if findutils_active { "active" } else { "unset" }
        );
        println!("sudo: {}", if sudo_active { "active" } else { "unset" });
        if coreutils_active {
            eprintln!("Tip: restore with 'oxidizr-arch restore coreutils --commit'.");
            eprintln!("Next: after validating workloads, you may fully switch by removing GNU packages with 'oxidizr-arch --commit replace coreutils'.");
        }
        if findutils_active {
            eprintln!("Tip: restore with 'oxidizr-arch restore findutils --commit'.");
            eprintln!("Next: after validating workloads, you may fully switch by removing GNU packages with 'oxidizr-arch --commit replace findutils'.");
        }
        if sudo_active {
            eprintln!("Tip: restore with 'oxidizr-arch restore sudo --commit'.");
            eprintln!("Next: after validating workloads, you may fully switch by removing GNU packages with 'oxidizr-arch --commit replace sudo'.");
        }
    }
    Ok(())
}
