use std::fs;

use crate::checks::Distribution;
use crate::error::Result;
use crate::logging::{audit_event_fields, AuditFields};

impl super::Worker {
    /// Get current distribution information
    pub fn distribution(&self) -> Result<Distribution> {
        let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let mut id: Option<String> = None;
        let mut id_like: Option<String> = None;

        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("ID=") {
                id = Some(rest.trim_matches('"').to_string());
            }
            if let Some(rest) = line.strip_prefix("ID_LIKE=") {
                id_like = Some(rest.trim_matches('"').to_string());
            }
        }

        Ok(Distribution {
            id: id.unwrap_or_else(|| "arch".to_string()),
            id_like: id_like.unwrap_or_default(),
            release: "rolling".to_string(),
        })
    }

    /// Check if official repositories (e.g., [extra]) are available
    pub fn extra_repo_available(&self) -> Result<bool> {
        // 1) Prefer a concise repo list to avoid false positives (e.g., NoExtract)
        if let Ok(out) = {
            tracing::debug!(cmd = %"pacman-conf --repo-list", "exec");
            let o = std::process::Command::new("pacman-conf")
                .args(["--repo-list"]) // lists repo names, one per line
                .output();
            if let Ok(ref o2) = o {
                tracing::debug!(status = ?o2.status.code(), "exit");
            }
            o
        } {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if out.status.success() {
                let found = stdout
                    .lines()
                    .map(|s| s.trim().to_ascii_lowercase())
                    .any(|l| l == "extra");
                let _ = audit_event_fields(
                    "worker",
                    "extra_repo_available.repo_list",
                    if found { "detected" } else { "not_detected" },
                    &AuditFields {
                        cmd: Some("pacman-conf --repo-list".to_string()),
                        rc: out.status.code(),
                        ..Default::default()
                    },
                );
                if found {
                    return Ok(true);
                }
                // Do not return false yet; fall through to additional heuristics
            }
        }

        // 2) Probe pacman sync DB for the 'extra' repo directly. Requires that callers refreshed (-Sy).
        if let Ok(status) = {
            tracing::debug!(cmd = %"pacman -Sl extra", "exec");
            let s = std::process::Command::new("pacman")
                .args(["-Sl", "extra"]) // list packages in 'extra'
                .status();
            if let Ok(ref st) = s {
                tracing::debug!(status = ?st.code(), "exit");
            }
            s
        } {
            let _ = audit_event_fields(
                "worker",
                "extra_repo_available.pacman_sl",
                if status.success() {
                    "detected"
                } else {
                    "not_detected"
                },
                &AuditFields {
                    cmd: Some("pacman -Sl extra".to_string()),
                    rc: status.code(),
                    ..Default::default()
                },
            );
            if status.success() {
                return Ok(true);
            }
        }

        // 3) Fallback to full config dump; look for an explicit [extra] section
        if let Ok(out) = {
            tracing::debug!(cmd = %"pacman-conf -l", "exec");
            let o = std::process::Command::new("pacman-conf")
                .args(["-l"]) // list configuration
                .output();
            if let Ok(ref o2) = o {
                tracing::debug!(status = ?o2.status.code(), "exit");
            }
            o
        } {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if out.status.success() {
                let found = stdout.to_ascii_lowercase().contains("[extra]");
                let _ = audit_event_fields(
                    "worker",
                    "extra_repo_available.conf_dump",
                    if found { "detected" } else { "not_detected" },
                    &AuditFields {
                        cmd: Some("pacman-conf -l".to_string()),
                        rc: out.status.code(),
                        ..Default::default()
                    },
                );
                if found {
                    return Ok(true);
                }
            }
        }

        // 4) Last resort: parse /etc/pacman.conf for a [extra] section
        let conf = fs::read_to_string("/etc/pacman.conf").unwrap_or_default();
        let found = conf.to_ascii_lowercase().contains("[extra]");
        let _ = audit_event_fields(
            "worker",
            "extra_repo_available.file_fallback",
            if found { "detected" } else { "not_detected" },
            &AuditFields {
                cmd: Some("/etc/pacman.conf".to_string()),
                ..Default::default()
            },
        );
        Ok(found)
    }
}
