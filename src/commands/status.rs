use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
struct StatusJson<'a> {
    coreutils: &'a str,
    findutils: &'a str,
    sudo: &'a str,
}

pub fn exec(root: &Path, json: bool) -> Result<(), String> {
    let check = |name: &str| -> bool {
        root.join("usr/bin").join(name)
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    };
    // Consider package active if ANY representative applet symlink exists
    let coreutils_active = ["ls", "cat", "echo", "mv"].iter().any(|n| check(n));
    let findutils_active = ["find", "xargs"].iter().any(|n| check(n));
    let sudo_active = check("sudo");

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
