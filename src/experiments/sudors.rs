use crate::checks::{is_supported_distro, Distribution};
use crate::error::{Error, Result};
use crate::experiments::util::{resolve_usrbin, restore_targets, verify_removed};
use crate::experiments::{check_download_prerequisites, SUDO_RS};
use crate::system::Worker;
use crate::state;
use std::fs;
use std::os::unix::fs::MetadataExt;
use crate::ui::progress;
use std::path::PathBuf;

pub struct SudoRsExperiment {
    name: String,
    package_name: String,
}

impl SudoRsExperiment {
    pub fn new() -> Self {
        Self {
            name: "sudo-rs".to_string(),
            package_name: SUDO_RS.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn check_compatible(&self, distro: &Distribution) -> Result<bool> {
        Ok(is_supported_distro(&distro.id))
    }

    pub fn enable(&self, worker: &Worker, assume_yes: bool, update_lists: bool) -> Result<()> {
        let _span =
            tracing::info_span!("sudors_enable", package = %self.package_name, update_lists)
                .entered();
        if update_lists {
            tracing::info!("Updating package lists...");
            worker.update_packages(assume_yes)?;
        }

        // Check prerequisites and handle prompts
        let reinstall = check_download_prerequisites(worker, &self.package_name, assume_yes)?;

        // Install package
        tracing::info!(event = "package_install", package = %self.package_name, "Installing package: {}", self.package_name);
        worker.install_package(&self.package_name, assume_yes, reinstall)?;
        if worker.check_installed(&self.package_name)? {
            tracing::info!(
                "✅ Expected: '{}' installed, Received: present",
                self.package_name
            );
        } else {
            tracing::error!(
                "❌ Expected: '{}' installed, Received: absent",
                self.package_name
            );
            return Err(Error::ExecutionFailed(format!(
                "❌ Expected: '{}' installed, Received: absent",
                self.package_name
            )));
        }

        // Replace sudo, su, visudo with binaries provided by sudo-rs
        let items = [
            ("sudo", self.resolve_target("sudo")),
            ("su", self.resolve_target("su")),
            ("visudo", PathBuf::from("/usr/sbin/visudo")),
        ];
        let pb = progress::new_bar(items.len() as u64);
        let _quiet_guard = if pb.is_some() {
            Some(progress::enable_symlink_quiet())
        } else {
            None
        };
        for (name, target) in items {
            tracing::info!("Preparing sudo-rs applet '{}'", name);

            let source = self.find_sudors_source(worker, name);
            let source = source.ok_or_else(|| {
                Error::ExecutionFailed(format!(
                    "Could not find installed sudo-rs binary for '{0}'. \
                     Checked: /usr/lib/cargo/bin/{0} and /usr/bin/{0}-rs. \
                     Hints: ensure 'sudo-rs' is installed and provides '{0}' on this distro.",
                    name
                ))
            })?;

            // Create a stable alias in /usr/bin so that readlink(1) shows '/usr/bin/<name>.sudo-rs'
            let alias = PathBuf::from(format!("/usr/bin/{}.sudo-rs", name));
            if pb.is_none() {
                tracing::info!(
                    "Creating alias for sudo-rs '{}': {} -> {}",
                    name,
                    alias.display(),
                    source.display()
                );
            }
            worker.replace_file_with_symlink(&source, &alias)?;
            // Verify alias symlink presence for visibility; treat mismatches as hard errors
            match std::fs::symlink_metadata(&alias) {
                Ok(m) if m.file_type().is_symlink() => {
                    if pb.is_none() {
                        tracing::info!(
                            "✅ Expected: '{}' alias symlink present, Received: symlink",
                            name
                        );
                    }
                }
                Ok(_) => {
                    return Err(Error::ExecutionFailed(format!(
                        "alias for '{}' not a symlink: {}",
                        name,
                        alias.display()
                    )));
                }
                Err(e) => {
                    return Err(Error::ExecutionFailed(format!(
                        "alias for '{}' missing: {} (err: {})",
                        name,
                        alias.display(),
                        e
                    )));
                }
            }

            if pb.is_none() {
                tracing::info!(
                    "Linking sudo-rs '{}' via alias: {} -> {}",
                    name,
                    target.display(),
                    alias.display()
                );
            }
            worker.replace_file_with_symlink(&alias, &target)?;
            // Verify target symlink presence; treat mismatches as hard errors
            match std::fs::symlink_metadata(&target) {
                Ok(m) if m.file_type().is_symlink() => {
                    if pb.is_none() {
                        tracing::info!(
                            "✅ Expected: '{}' linked via alias, Received: symlink",
                            name
                        );
                    }
                }
                Ok(_) => {
                    return Err(Error::ExecutionFailed(format!(
                        "target for '{}' not a symlink: {}",
                        name,
                        target.display()
                    )));
                }
                Err(e) => {
                    return Err(Error::ExecutionFailed(format!(
                        "target for '{}' missing: {} (err: {})",
                        name,
                        target.display(),
                        e
                    )));
                }
            }

            // Update bar after finishing both alias and target for this name
            progress::set_msg_and_inc(&pb, format!("Linking {}", name));
        }
        progress::finish(pb);
        // Post-enable verifier: setuid, ownership, PAM, smoke test
        if let Err(e) = self.verify_post_enable(worker) {
            // Revert on failure
            let targets = vec![
                self.resolve_target("sudo"),
                self.resolve_target("su"),
                PathBuf::from("/usr/sbin/visudo"),
            ];
            let _ = crate::experiments::util::restore_targets(worker, &targets);
            // Remove alias symlinks
            for n in ["sudo", "su", "visudo"] {
                let alias = PathBuf::from(format!("/usr/bin/{}.sudo-rs", n));
                let _ = fs::remove_file(&alias);
            }
            return Err(e);
        }

        // Persist state: mark enabled and record managed targets
        let managed = vec![
            self.resolve_target("sudo"),
            self.resolve_target("su"),
            PathBuf::from("/usr/sbin/visudo"),
        ];
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
            tracing::info_span!("sudors_disable", package = %self.package_name, update_lists)
                .entered();
        if update_lists {
            tracing::info!("Updating package lists...");
            worker.update_packages(assume_yes)?;
        }

        // Restore original binaries (fail fast on mismatches)
        let targets = vec![
            self.resolve_target("sudo"),
            self.resolve_target("su"),
            PathBuf::from("/usr/sbin/visudo"),
        ];
        restore_targets(worker, &targets)?;
        // Persist state: mark disabled and remove managed targets
        let _ = state::set_enabled(
            worker.state_dir_override.as_deref(),
            worker.dry_run,
            self.name(),
            false,
            &targets,
        );
        // Verify restored (not a symlink)
        for (name, target) in [
            ("sudo", &targets[0]),
            ("su", &targets[1]),
            ("visudo", &targets[2]),
        ] {
            match std::fs::symlink_metadata(target) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(Error::ExecutionFailed(format!(
                        "{} was expected restored to non-symlink but is still a symlink: {}",
                        name,
                        target.display()
                    )));
                }
                Ok(_) => {
                    tracing::info!(
                        "✅ Expected: '{}' restored to non-symlink, Received: non-symlink",
                        name
                    );
                }
                Err(e) => {
                    return Err(Error::ExecutionFailed(format!(
                        "{} missing after restore: {} (err: {})",
                        name,
                        target.display(),
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn remove(&self, worker: &Worker, assume_yes: bool, update_lists: bool) -> Result<()> {
        let _span =
            tracing::info_span!("sudors_remove", package = %self.package_name, update_lists)
                .entered();
        // First restore GNU tools
        self.disable(worker, assume_yes, update_lists)?;

        // Then remove the package
        tracing::info!(event = "package_remove", package = %self.package_name, "Removing package: {}", self.package_name);
        worker.remove_package(&self.package_name, assume_yes)?;

        // Verify absence
        verify_removed(worker, &self.package_name)?;

        Ok(())
    }

    pub fn list_targets(&self) -> Vec<PathBuf> {
        vec![
            self.resolve_target("sudo"),
            self.resolve_target("su"),
            PathBuf::from("/usr/sbin/visudo"),
        ]
    }

    fn find_sudors_source(&self, worker: &Worker, name: &str) -> Option<PathBuf> {
        // Always resolve to sudo-rs-provided binaries. Do not fall back to the system 'sudo'.
        // Prefer explicit locations, then consult PATH for '*-rs'.
        let rs_name = format!("{}-rs", name);
        let candidates = [
            PathBuf::from(format!("/usr/lib/cargo/bin/{}", name)),
            PathBuf::from(format!("/usr/bin/{}", rs_name)),
        ];

        for c in candidates {
            tracing::debug!("checking sudo-rs candidate for '{}': {}", name, c.display());
            if c.exists() {
                return Some(c);
            }
        }

        if let Ok(Some(path)) = worker.which(&rs_name) {
            tracing::debug!("found sudo-rs on PATH for '{}': {}", name, path.display());
            return Some(path);
        }

        None
    }

    fn resolve_target(&self, filename: &str) -> PathBuf {
        resolve_usrbin(filename)
    }

    fn verify_post_enable(&self, worker: &Worker) -> Result<()> {
        // 1) Check real binary ownership and setuid bits for sudo, su, visudo
        for (name, _target) in [
            ("sudo", self.resolve_target("sudo")),
            ("su", self.resolve_target("su")),
            ("visudo", PathBuf::from("/usr/sbin/visudo")),
        ] {
            // Follow alias to real binary
            let alias = PathBuf::from(format!("/usr/bin/{}.sudo-rs", name));
            let real = fs::canonicalize(&alias).map_err(|e| Error::ExecutionFailed(format!(
                "failed to resolve real binary for {} via {}: {}",
                name,
                alias.display(),
                e
            )))?;
            let meta = fs::metadata(&real)?;
            if meta.uid() != 0 || meta.gid() != 0 || (meta.mode() & 0o4000) == 0 {
                return Err(Error::ExecutionFailed(format!(
                    "post-enable verifier failed for {} (real: {}): require uid=0,gid=0,setuid bit",
                    name,
                    real.display()
                )));
            }
        }
        // 2) PAM file exists
        if fs::metadata("/etc/pam.d/sudo").is_err() {
            return Err(Error::ExecutionFailed(
                "PAM file /etc/pam.d/sudo not found after sudo-rs enable".into(),
            ));
        }
        // 3) Smoke test
        if let Some(user) = &worker.sudo_smoke_user {
            let status = std::process::Command::new("sudo")
                .args(["-n", "-u", user, "true"])
                .status()
                .map_err(|e| Error::ExecutionFailed(format!("failed to run sudo smoke test: {}", e)))?;
            if !status.success() {
                return Err(Error::ExecutionFailed(
                    "sudo -n true smoke test failed for configured user".into(),
                ));
            }
        } else {
            tracing::warn!("sudo-rs: skipping sudo smoke test; provide --sudo-smoke-user to enable");
        }
        Ok(())
    }
}
