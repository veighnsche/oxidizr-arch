use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::args::{Package, ParityLevel};
use crate::util::selinux::selinux_enabled;
use crate::commands::replace_utils::{link_points_to_exec, resolve_source_bin};
use oxidizr_cli_core::{DistroAdapter, PackageKind};
use oxidizr_cli_core::packages::{coreutils_critical_set, coreutils_selinux_set, static_fallback_applets};
use oxidizr_cli_core::coverage2::coverage_preflight;

pub struct ReplaceReady {
    pub provider: &'static str,
    pub selinux_on: bool,
}

pub fn enforce_replace_parity<A: DistroAdapter>(
    adapter: &A,
    root: &Path,
    pkg: Package,
    parity: ParityLevel,
    offline: bool,
    use_local: &Option<PathBuf>,
) -> Result<ReplaceReady, String> {
    let provider = match pkg { Package::Sudo => "sudo-rs", _ => "uutils" };
    let selinux_on = selinux_enabled(root);

    // Strict mode: require full distro coverage via coverage_preflight
    if matches!(parity, ParityLevel::Strict) {
        if let Some(kind) = match pkg {
            Package::Coreutils => Some(PackageKind::Coreutils),
            Package::Findutils => Some(PackageKind::Findutils),
            Package::Sudo => None,
        } {
            let source_bin = if offline {
                use_local.clone().unwrap()
            } else {
                resolve_source_bin(pkg)
            };
            if let Err(missing) = coverage_preflight(adapter, root, kind, &source_bin) {
                return Err(format!(
                    "replace parity(strict) failed for {:?}: missing: {}",
                    pkg,
                    missing.join(", ")
                ));
            }
        }
        // Summary
        eprintln!(
            "{}",
            json!({
                "event":"replace.exec.summary",
                "package": format!("{:?}", pkg),
                "provider": provider,
                "selinux_enabled": selinux_on,
                "parity_threshold": format!("{:?}", parity),
                "parity_ok": true,
                "critical_missing": [],
                "selinux_missing": [],
            })
        );
        println!(
            "[READY] replace {:?}: provider={} parity=OK ({})",
            pkg,
            provider,
            format!("{:?}", parity).to_lowercase()
        );
        return Ok(ReplaceReady { provider, selinux_on });
    }

    // Standard / Selinux / None
    let (critical_set, selinux_set): (Vec<String>, Vec<String>) = match pkg {
        Package::Coreutils => (coreutils_critical_set(), coreutils_selinux_set()),
        Package::Findutils => (static_fallback_applets(PackageKind::Findutils), vec![]),
        Package::Sudo => (vec!["sudo".to_string()], vec![]),
    };

    let mut crit_missing: Vec<String> = critical_set
        .iter()
        .filter(|n| !link_points_to_exec(root, n))
        .cloned()
        .collect();
    crit_missing.sort();
    crit_missing.dedup();

    let mut se_missing: Vec<String> = selinux_set
        .iter()
        .filter(|n| !link_points_to_exec(root, n))
        .cloned()
        .collect();
    se_missing.sort();
    se_missing.dedup();

    let require_se = matches!(parity, ParityLevel::Selinux)
        || (matches!(parity, ParityLevel::Standard) && selinux_on);
    let fail = (!matches!(parity, ParityLevel::None))
        && (!crit_missing.is_empty() || (require_se && !se_missing.is_empty()));

    if fail {
        let mut hint = String::new();
        if require_se && !se_missing.is_empty() {
            hint = "Hint: install a SELinux-enabled uutils build providing uu-chcon/uu-runcon (e.g., AUR variant).".to_string();
        }
        return Err(format!(
            "replace parity({:?}) failed for {:?}: missing critical=[{}] selinux=[{}]. {}",
            parity,
            pkg,
            crit_missing.join(", "),
            se_missing.join(", "),
            hint
        ));
    }

    eprintln!(
        "{}",
        json!({
            "event":"replace.exec.summary",
            "package": format!("{:?}", pkg),
            "provider": provider,
            "selinux_enabled": selinux_on,
            "parity_threshold": format!("{:?}", parity),
            "parity_ok": true,
            "critical_missing": [],
            "selinux_missing": [],
        })
    );
    println!(
        "[READY] replace {:?}: provider={} parity=OK ({})",
        pkg,
        provider,
        format!("{:?}", parity).to_lowercase()
    );

    Ok(ReplaceReady { provider, selinux_on })
}

/// Filter post-verify names according to parity policy to avoid false failures
pub fn filter_postverify_names(
    names: Vec<String>,
    root: &Path,
    pkg: Package,
    parity: ParityLevel,
) -> Vec<String> {
    if matches!(pkg, Package::Findutils) {
        let allow = static_fallback_applets(PackageKind::Findutils);
        return names.into_iter().filter(|n| allow.contains(n)).collect();
    }
    if matches!(pkg, Package::Sudo) {
        return vec!["sudo".to_string()].into_iter().filter(|n| names.contains(n)).collect();
    }

    // Coreutils
    let selinux_on = selinux_enabled(root);
    let crit = coreutils_critical_set();
    let se = coreutils_selinux_set();
    match parity {
        ParityLevel::Strict => names,
        ParityLevel::Selinux => names.into_iter().filter(|n| crit.contains(n) || se.contains(n)).collect(),
        ParityLevel::Standard => {
            if selinux_on {
                names.into_iter().filter(|n| crit.contains(n) || se.contains(n)).collect()
            } else {
                names.into_iter().filter(|n| crit.contains(n)).collect()
            }
        }
        ParityLevel::None => names.into_iter().filter(|n| crit.contains(n)).collect(),
    }
}
