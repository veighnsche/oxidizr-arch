use std::collections::HashSet;
use std::path::Path;

use serde_json::json;

use crate::cli::args::{Package, ParityLevel};
use crate::util::selinux::selinux_enabled;
use oxidizr_cli_core::packages::{coreutils_critical_set, coreutils_selinux_set, static_fallback_applets};
use oxidizr_cli_core::PackageKind;

/// Compute and emit parity summary for `use`.
/// Returns (parity_ok, critical_missing, selinux_missing).
pub fn emit_use_parity_summary(
    root: &Path,
    package: Package,
    parity: ParityLevel,
    allow_missing: Option<String>,
    applets: &[String],
    skipped: &[String],
    linked_count: usize,
) -> (bool, Vec<String>, Vec<String>) {
    let provider = match package { Package::Sudo => "sudo-rs", _ => "uutils" };
    let selinux_on = selinux_enabled(root);
    let allow_set: HashSet<String> = allow_missing
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let covered: HashSet<String> = applets
        .iter()
        .filter(|a| !skipped.iter().any(|s| s == *a))
        .cloned()
        .collect();

    let (critical_set, selinux_set): (Vec<String>, Vec<String>) = match package {
        Package::Coreutils => (coreutils_critical_set(), coreutils_selinux_set()),
        Package::Findutils => (static_fallback_applets(PackageKind::Findutils), vec![]),
        Package::Sudo => (vec!["sudo".to_string()], vec![]),
    };

    let mut critical_missing: Vec<String> = critical_set
        .iter()
        .filter(|c| !covered.contains(*c))
        .cloned()
        .collect();
    critical_missing.sort();
    critical_missing.dedup();

    let critical_missing_eval: Vec<String> = critical_missing
        .iter()
        .filter(|c| !allow_set.contains((*c).as_str()))
        .cloned()
        .collect();

    let mut selinux_missing: Vec<String> = selinux_set
        .iter()
        .filter(|c| !covered.contains(*c))
        .cloned()
        .collect();
    selinux_missing.sort();
    selinux_missing.dedup();

    let parity_ok = match parity {
        ParityLevel::None => true,
        ParityLevel::Standard => critical_missing_eval.is_empty() && (!selinux_on || selinux_missing.is_empty()),
        ParityLevel::Selinux => critical_missing_eval.is_empty() && selinux_missing.is_empty(),
        ParityLevel::Strict => critical_missing_eval.is_empty() && (!selinux_on || selinux_missing.is_empty()),
    };

    // JSON summary (stderr)
    eprintln!(
        "{}",
        json!({
            "event":"use.exec.summary",
            "package": format!("{:?}", package),
            "provider": provider,
            "selinux_enabled": selinux_on,
            "parity_threshold": format!("{:?}", parity),
            "parity_ok": parity_ok,
            "critical_missing": critical_missing,
            "selinux_missing": selinux_missing,
            "skipped": skipped,
        })
    );

    // Human-friendly summary (stdout)
    let pkg_name = format!("{:?}", package).to_lowercase();
    let status = if parity_ok { "OK" } else { "WARN" };
    let skipped_text = if skipped.is_empty() { "none".to_string() } else { skipped.join(", ") };
    let sel = if selinux_on { "enabled" } else { "disabled" };
    println!(
        "[{}] use {}: provider={} linked={} skipped=[{}] parity={} ({}) selinux={}",
        status,
        pkg_name,
        provider,
        linked_count,
        skipped_text,
        if parity_ok { "OK" } else { "BELOW" },
        format!("{:?}", parity).to_lowercase(),
        sel,
    );

    (parity_ok, critical_missing, selinux_missing)
}
