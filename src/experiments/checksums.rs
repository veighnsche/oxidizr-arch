use crate::checks::{is_supported_distro, Distribution};
use crate::error::{Error, Result};
use crate::experiments::constants::CHECKSUM_BINS;
use crate::experiments::util::{
    create_symlinks, log_applets_summary, resolve_usrbin, restore_targets,
};
use crate::experiments::{check_download_prerequisites, UUTILS_COREUTILS};
use crate::logging::{audit_event_fields, AuditFields};
use crate::state;
use crate::system::Worker;
use std::path::PathBuf;

pub struct ChecksumsExperiment {
    name: String,
    unified_binary: Option<PathBuf>,
    bin_directory: PathBuf,
}

impl ChecksumsExperiment {
    pub fn new() -> Self {
        Self {
            name: "checksums".to_string(),
            unified_binary: Some(PathBuf::from("/usr/bin/coreutils")),
            bin_directory: PathBuf::from("/usr/lib/uutils/coreutils"),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn check_compatible(&self, distro: &Distribution) -> Result<bool> {
        Ok(is_supported_distro(&distro.id))
    }

    pub fn enable(&self, worker: &Worker, assume_yes: bool, update_lists: bool) -> Result<()> {
        let _span = tracing::info_span!("checksums_enable").entered();

        if update_lists {
            tracing::info!("Updating package lists...");
            worker.update_packages(assume_yes)?;
        }

        // First attempt discovery using existing installation/state
        let (mut links, mut skipped) = self.discover_present(worker)?;

        // If nothing is available, proactively install provider package (default: uutils-coreutils)
        if links.is_empty() {
            let provider = worker
                .package_override
                .as_ref()
                .cloned()
                .unwrap_or_else(|| UUTILS_COREUTILS.to_string());
            tracing::info!(
                "No checksum applets found in current system state; ensuring '{}' is installed",
                provider
            );
            let reinstall = check_download_prerequisites(worker, &provider, assume_yes)?;
            tracing::info!("Installing package: {}", provider);
            worker.install_package(&provider, assume_yes, reinstall)?;
            // Re-discover after install
            let rediscovered = self.discover_present(worker)?;
            links = rediscovered.0;
            skipped = rediscovered.1;
        }
        if !links.is_empty() {
            tracing::info!("flip-checksums: linking {} checksum applet(s)", links.len());
        }
        for s in &skipped {
            tracing::warn!(
                "flip-checksums: checksum tool '{}' not provided by uutils-coreutils on this distro/build; skipping",
                s
            );
        }
        if links.is_empty() {
            let _ = audit_event_fields(
                "experiments",
                "nothing_to_link",
                "checksums",
                &AuditFields {
                    suite: Some("checksums".to_string()),
                    ..Default::default()
                },
            );
            return Err(Error::NothingToLink(
                "no checksum applets discovered after ensuring provider".into(),
            ));
        }

        log_applets_summary("checksums", &links, 8);
        create_symlinks(worker, &links, |name| self.resolve_target(name))?;
        // Persist state: mark enabled and record managed checksum targets
        let managed: Vec<PathBuf> = links.iter().map(|(n, _)| self.resolve_target(n)).collect();
        let _ = state::set_enabled(
            worker.state_dir_override.as_deref(),
            worker.dry_run,
            self.name(),
            true,
            &managed,
        );
        Ok(())
    }

    pub fn disable(&self, worker: &Worker, _assume_yes: bool, _update_lists: bool) -> Result<()> {
        let _span = tracing::info_span!("checksums_disable").entered();
        let targets: Vec<PathBuf> = CHECKSUM_BINS
            .iter()
            .map(|n| self.resolve_target(n))
            .collect();
        restore_targets(worker, &targets)?;
        // Persist state: mark disabled and remove managed targets
        let _ = state::set_enabled(
            worker.state_dir_override.as_deref(),
            worker.dry_run,
            self.name(),
            false,
            &targets,
        );
        Ok(())
    }

    pub fn remove(&self, worker: &Worker, assume_yes: bool, update_lists: bool) -> Result<()> {
        // Nothing to uninstall; simply restore, and clarify intent in logs
        tracing::info!("checksums: remove -> disable only (no package removal performed)");
        self.disable(worker, assume_yes, update_lists)
    }

    pub fn list_targets(&self) -> Vec<PathBuf> {
        CHECKSUM_BINS
            .iter()
            .map(|n| self.resolve_target(n))
            .collect()
    }

    fn discover_present(&self, worker: &Worker) -> Result<(Vec<(String, PathBuf)>, Vec<String>)> {
        let mut applets: Vec<(String, PathBuf)> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();

        // Apply overrides
        let effective_unified = worker
            .unified_binary_override
            .as_ref()
            .cloned()
            .or_else(|| self.unified_binary.clone());
        let effective_bin_dir = worker
            .bin_dir_override
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.bin_directory.clone());

        // Prefer unified dispatcher if present
        let unified_path = if let Some(ref path) = effective_unified {
            if path.exists() {
                Some(path.clone())
            } else if let Ok(Some(found)) = worker.which("coreutils") {
                Some(found)
            } else {
                None
            }
        } else {
            None
        };

        for name in CHECKSUM_BINS {
            if let Some(ref unified) = unified_path {
                applets.push((name.to_string(), unified.clone()));
                continue;
            }
            // Try per-applet
            let candidates = [
                effective_bin_dir.join(name),
                PathBuf::from(format!("/usr/bin/uu-{}", name)),
                PathBuf::from(format!("/usr/lib/cargo/bin/coreutils/{}", name)),
                PathBuf::from(format!("/usr/lib/cargo/bin/{}", name)),
            ];
            if let Some(found) = candidates.iter().find(|p| p.exists()) {
                applets.push((name.to_string(), found.clone()));
            } else {
                skipped.push(name.to_string());
            }
        }

        Ok((applets, skipped))
    }

    fn resolve_target(&self, filename: &str) -> PathBuf {
        resolve_usrbin(filename)
    }
}
