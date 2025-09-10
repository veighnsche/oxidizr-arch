use crate::checks::{is_supported_distro, Distribution};
use crate::error::{Error, Result};
use crate::experiments::constants::CHECKSUM_BINS;
use crate::experiments::util::{
    create_symlinks, log_applets_summary, resolve_usrbin, restore_targets, verify_removed,
};
use crate::experiments::{check_download_prerequisites, UUTILS_COREUTILS};
use crate::system::Worker;
use crate::state;
use std::path::PathBuf;

// Coreutils bins list (moved under assets per DELTA)
const COREUTILS_BINS_LIST: &str = include_str!("../assets/coreutils-bins.txt");

// Binaries we must not replace to keep packaging tools functional (e.g., makepkg)
const PRESERVE_BINS: &[&str] = &[
    "b2sum",
    "md5sum",
    "sha1sum",
    "sha224sum",
    "sha256sum",
    "sha384sum",
    "sha512sum",
];

pub struct CoreutilsExperiment {
    name: String,
    package_name: String,
    unified_binary: Option<PathBuf>,
    bin_directory: PathBuf,
}

impl CoreutilsExperiment {
    pub fn new() -> Self {
        Self {
            name: "coreutils".to_string(),
            package_name: UUTILS_COREUTILS.to_string(),
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
        let _span = tracing::info_span!(
            "coreutils_enable",
            package = %self.package_name,
            update_lists,
        )
        .entered();
        if update_lists {
            tracing::info!("Updating package lists...");
            worker.update_packages(assume_yes)?;
        }

        // Effective package with optional override
        let effective_package = worker
            .package_override
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.package_name.clone());

        // Check prerequisites and handle prompts
        let reinstall = check_download_prerequisites(worker, &effective_package, assume_yes)?;

        // Install package (honor reinstall request)
        tracing::info!(event = "package_install", package = %effective_package, "Installing package: {}", effective_package);
        worker.install_package(&effective_package, assume_yes, reinstall)?;

        // Discover and link applets
        let applets = self.discover_applets(worker)?;
        if applets.is_empty() {
            tracing::error!(
                "❌ Expected: at least 1 coreutils applet discovered after install; Received: 0"
            );
            return Err(Error::ExecutionFailed(format!(
                "❌ Expected: coreutils applets discovered; Received: 0. Ensure {} is installed correctly.",
                self.package_name
            )));
        }
        tracing::info!(
            "✅ Expected: coreutils applets discovered; Received: {}",
            applets.len()
        );

        // Build link plan: always exclude checksum applets for safety; those are handled by the
        // dedicated 'checksums' experiment.
        let to_link: Vec<(String, PathBuf)> = applets
            .into_iter()
            .filter(|(name, _)| !PRESERVE_BINS.contains(&name.as_str()))
            .collect();

        log_applets_summary("coreutils", &to_link, 8);
        create_symlinks(worker, &to_link, |name| self.resolve_target(name))?;
        // Persist state: mark enabled and record managed (non-checksum) targets
        let managed: Vec<PathBuf> = to_link
            .iter()
            .map(|(n, _)| self.resolve_target(n))
            .collect();
        let _ = state::set_enabled(
            worker.state_dir_override.as_deref(),
            worker.dry_run,
            self.name(),
            true,
            &managed,
        );

        Ok(())
    }

    pub fn disable(&self, worker: &Worker, assume_yes: bool, update_lists: bool) -> Result<()> {
        let _span =
            tracing::info_span!("coreutils_disable", package = %self.package_name, update_lists)
                .entered();
        if update_lists {
            tracing::info!("Updating package lists...");
            worker.update_packages(assume_yes)?;
        }

        // Restore only non-checksum coreutils applets (checksums are handled by the dedicated experiment)
        let mut targets: Vec<PathBuf> = Vec::new();
        for line in COREUTILS_BINS_LIST.lines() {
            let filename = line.trim();
            if filename.is_empty() {
                continue;
            }
            if PRESERVE_BINS.contains(&filename) {
                continue;
            }
            let target = self.resolve_target(filename);
            targets.push(target);
        }
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
        let _span =
            tracing::info_span!("coreutils_remove", package = %self.package_name, update_lists)
                .entered();
        // First restore GNU tools
        self.disable(worker, assume_yes, update_lists)?;

        // Preflight: refuse to remove if checksum applets appear to be linked (to avoid breaking active checksum links)
        // Ask the user to disable the 'checksums' experiment first.
        let mut active_checksum_links = Vec::new();
        for name in CHECKSUM_BINS {
            let target = self.resolve_target(name);
            if let Ok(meta) = std::fs::symlink_metadata(&target) {
                if meta.file_type().is_symlink() {
                    active_checksum_links.push(target);
                }
            }
        }
        if !active_checksum_links.is_empty() {
            tracing::error!(
                "❌ Refusing to remove '{}' while checksum applets are still linked. Disable 'checksums' experiment first.",
                self.package_name
            );
            return Err(Error::ExecutionFailed(
                "checksums experiment appears active; run 'oxidizr-arch disable --experiments checksums' first".into()
            ));
        }

        // Then remove the package
        tracing::info!(event = "package_remove", package = %self.package_name, "Removing package: {}", self.package_name);
        worker.remove_package(&self.package_name, assume_yes)?;

        // Verify absence explicitly
        verify_removed(worker, &self.package_name)?;

        Ok(())
    }

    pub fn list_targets(&self) -> Vec<PathBuf> {
        let mut targets = Vec::new();
        for line in COREUTILS_BINS_LIST.lines() {
            let filename = line.trim();
            if filename.is_empty() {
                continue;
            }
            if PRESERVE_BINS.contains(&filename) {
                continue;
            }
            targets.push(self.resolve_target(filename));
        }
        targets
    }

    fn discover_applets(&self, worker: &Worker) -> Result<Vec<(String, PathBuf)>> {
        let mut applets = Vec::new();

        // Determine effective unified binary and bin directory from overrides
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

        // Check for unified binary first (effective)
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

        if let Some(unified) = unified_path {
            tracing::info!("Using unified coreutils binary at: {}", unified.display());
            // Use unified binary for all applets
            for line in COREUTILS_BINS_LIST.lines() {
                let name = line.trim();
                if !name.is_empty() {
                    applets.push((name.to_string(), unified.clone()));
                }
            }
        } else {
            tracing::warn!("Unified dispatcher not available; falling back to per-applet binaries");
            // Try to find individual binaries
            let mut skipped: Vec<String> = Vec::new();
            for line in COREUTILS_BINS_LIST.lines() {
                let name = line.trim();
                if name.is_empty() {
                    continue;
                }

                // Try various locations
                let candidates = [
                    effective_bin_dir.join(name),
                    PathBuf::from(format!("/usr/bin/uu-{}", name)),
                    PathBuf::from(format!("/usr/lib/cargo/bin/coreutils/{}", name)),
                    PathBuf::from(format!("/usr/lib/cargo/bin/{}", name)),
                ];

                if let Some(found) = candidates.iter().find(|p| p.exists()) {
                    applets.push((name.to_string(), found.clone()));
                } else if let Ok(Some(path)) = worker.which(name) {
                    applets.push((name.to_string(), path));
                } else {
                    skipped.push(name.to_string());
                }
            }
            for s in &skipped {
                tracing::warn!(event = "skip_applet", target = %format!("/usr/bin/{}", s), reason = "not provided by provider or not found in known locations", "Skipping applet '{}' (not found)", s);
            }
        }

        Ok(applets)
    }

    fn resolve_target(&self, filename: &str) -> PathBuf {
        resolve_usrbin(filename)
    }
}
