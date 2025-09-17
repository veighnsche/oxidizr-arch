use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::json;

/// Ensure the replacement package is installed on a live root.
/// Emits JSON events for package manager operations.
pub fn ensure_replacement_installed(root: &Path, rs_pkg: &str, live_root: bool) -> Result<(), String> {
    if !live_root {
        eprintln!(
            "[info] skipping pacman/paru install under non-live root: {}",
            root.display()
        );
        return Ok(());
    }

    // Check if already installed
    let st = Command::new("pacman")
        .args(["-Qi", rs_pkg])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if matches!(st, Ok(s) if s.success()) {
        return Ok(());
    }

    let mut tried = Vec::new();
    let mut ok = false;
    let mut last_code = 1;
    let mut last_stderr_tail = String::new();

    // Try pacman first (official)
    tried.push(format!("pacman -S --noconfirm {}", rs_pkg));
    let mut cmd = Command::new("pacman");
    cmd.args(["-S", "--noconfirm", rs_pkg]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    if let Ok(out) = cmd.output() {
        last_code = out.status.code().unwrap_or(1);
        last_stderr_tail = String::from_utf8_lossy(&out.stderr)
            .chars()
            .rev()
            .take(400)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        eprintln!(
            "{}",
            json!({
                "event":"pm.install","pm":{"tool":"pacman","args":["-S","--noconfirm",rs_pkg],"package":rs_pkg},
                "exit_code": last_code,
                "stderr_tail": last_stderr_tail
            })
        );
        ok = out.status.success();
    }

    // Fallback to paru (AUR) if available
    if !ok {
        tried.push(format!("paru -S --noconfirm {}", rs_pkg));
        if let Ok(paru_bin) = which::which("paru") {
            let mut cmd = Command::new(paru_bin);
            cmd.args(["-S", "--noconfirm", rs_pkg]);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            if let Ok(out) = cmd.output() {
                last_code = out.status.code().unwrap_or(1);
                last_stderr_tail = String::from_utf8_lossy(&out.stderr)
                    .chars()
                    .rev()
                    .take(400)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>();
                eprintln!(
                    "{}",
                    json!({
                        "event":"pm.install","pm":{"tool":"paru","args":["-S","--noconfirm",rs_pkg],"package":rs_pkg},
                        "exit_code": last_code,
                        "stderr_tail": last_stderr_tail
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

    // Fallback as root with OXI_AUR_HELPER_USER
    if !ok {
        if let Ok(helper_user) = std::env::var("OXI_AUR_HELPER_USER") {
            if which::which("sudo").is_ok() && which::which("paru").is_ok() {
                let mut cmd = Command::new("sudo");
                cmd.args(["-u", &helper_user, "paru", "-S", "--noconfirm", rs_pkg]);
                cmd.stdin(Stdio::null());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                tried.push(format!("sudo -u {} paru -S --noconfirm {}", helper_user, rs_pkg));
                if let Ok(out) = cmd.output() {
                    last_code = out.status.code().unwrap_or(1);
                    last_stderr_tail = String::from_utf8_lossy(&out.stderr)
                        .chars()
                        .rev()
                        .take(400)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>();
                    eprintln!(
                        "{}",
                        json!({
                            "event":"pm.install","pm":{"tool":"sudo -u","user":helper_user,"args":["paru","-S","--noconfirm",rs_pkg],"package":rs_pkg},
                            "exit_code": last_code,
                            "stderr_tail": last_stderr_tail
                        })
                    );
                    ok = out.status.success();
                }
            }
        }
    }

    if !ok {
        let mut msg = format!(
            "failed to install {} (tried: {}; last_code={})",
            rs_pkg,
            tried.join("; "),
            last_code
        );
        if rs_pkg == "uutils-findutils-bin" && last_stderr_tail.to_lowercase().contains("root") {
            msg.push_str(". AUR helper refused to run as root. Install 'uutils-findutils-bin' as a non-root user with your AUR helper, then rerun: 'oxidizr-arch --commit use findutils'.");
        }
        return Err(msg);
    }

    Ok(())
}
