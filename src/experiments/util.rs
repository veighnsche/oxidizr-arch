use crate::error::{Error, Result};
use crate::logging::{audit_event_fields, AuditFields};
use crate::system::Worker;
use crate::ui::progress;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[cfg(feature = "switchyard")]
use switchyard::{
    ApplyMode, AuditSink, FactsEmitter, LinkRequest, PlanInput, Policy, RestoreRequest, Switchyard,
};

#[cfg(feature = "switchyard")]
struct NullFacts;
#[cfg(feature = "switchyard")]
impl FactsEmitter for NullFacts {}

#[cfg(feature = "switchyard")]
struct NullAudit;
#[cfg(feature = "switchyard")]
impl AuditSink for NullAudit {}

/// Resolve a target path under /usr/bin
pub fn resolve_usrbin(filename: &str) -> PathBuf {
    Path::new("/usr/bin").join(filename)
}

/// Create symlinks for (filename -> src) applets, using a target resolver.
/// Adds detailed logs and wraps errors with src/target context.
pub fn create_symlinks<F>(worker: &Worker, applets: &[(String, PathBuf)], resolve: F) -> Result<()>
where
    F: Fn(&str) -> PathBuf,
{
    // Optional: run via switchyard facade when feature is enabled
    #[cfg(feature = "switchyard")]
    {
        return create_symlinks_via_switchyard(worker, applets, resolve);
    }

    let mut pb = progress::new_bar(applets.len() as u64);
    let _quiet_guard = if pb.is_some() {
        Some(progress::enable_symlink_quiet())
    } else {
        None
    };
    let total = applets.len().max(1);
    for (idx, (filename, src)) in applets.iter().enumerate() {
        let target = resolve(filename);
        tracing::trace!(step = "symlink_item", filename = %filename, src = %src.display(), target = %target.display());
        // When progress bar is active, avoid noisy per-item info logs
        if pb.is_none() {
            tracing::info!("Symlinking {} -> {}", src.display(), target.display());
        }
        // Structured audit: link_started
        let _ = audit_event_fields(
            "symlink",
            "link_started",
            "begin",
            &AuditFields {
                source: Some(src.display().to_string()),
                target: Some(target.display().to_string()),
                ..Default::default()
            },
        );
        // Emit host progress protocol line for v1 host bar
        progress::emit_host_pb(idx + 1, total, &format!("Linking {}", filename));
        let t0 = Instant::now();
        if let Err(e) = worker.replace_file_with_symlink(src, &target) {
            tracing::error!(
                "❌ Failed to create symlink: src={} -> target={}: {}",
                src.display(),
                target.display(),
                e
            );
            // Clear the bar on error for better UX
            progress::finish(pb.take());
            return Err(Error::ExecutionFailed(format!(
                "failed to symlink {} -> {}: {}",
                src.display(),
                target.display(),
                e
            )));
        }
        let elapsed_ms = t0.elapsed().as_millis() as u64;
        tracing::debug!(
            "link_done: {} -> {} ({} ms)",
            src.display(),
            target.display(),
            elapsed_ms
        );
        // Structured audit: link_done
        let _ = audit_event_fields(
            "symlink",
            "link_done",
            "success",
            &AuditFields {
                source: Some(src.display().to_string()),
                target: Some(target.display().to_string()),
                duration_ms: Some(elapsed_ms),
                ..Default::default()
            },
        );
        // Update progress after a successful link
        progress::set_msg_and_inc(&pb, format!("Linking {}", filename));
    }
    // Finish the bar if present
    progress::finish(pb);
    Ok(())
}

#[cfg(feature = "switchyard")]
fn create_symlinks_via_switchyard<F>(
    worker: &Worker,
    applets: &[(String, PathBuf)],
    resolve: F,
) -> Result<()>
where
    F: Fn(&str) -> PathBuf,
{
    let policy = Policy {
        allow_roots: vec![PathBuf::from("/usr")],
        forbid_paths: vec![],
        strict_ownership: worker.strict_ownership,
        force_untrusted_source: worker.force_override_untrusted,
        force_restore_best_effort: worker.force_restore_best_effort,
    };
    let sx = Switchyard::new(NullFacts, NullAudit, policy);
    let mut input = PlanInput::default();
    for (filename, src) in applets.iter() {
        let target = resolve(filename);
        input.link.push(LinkRequest {
            source: src.clone(),
            target,
        });
    }
    let plan = sx.plan(input);
    let pre = sx.preflight(&plan);
    if !pre.ok {
        return Err(Error::ExecutionFailed(format!(
            "preflight failed: {}",
            pre.stops.join(", ")
        )));
    }
    let mode = if worker.dry_run {
        ApplyMode::DryRun
    } else {
        ApplyMode::Commit
    };
    let rep = sx.apply(&plan, mode);
    if !rep.errors.is_empty() {
        return Err(Error::ExecutionFailed(format!(
            "apply errors: {}",
            rep.errors.join(", ")
        )));
    }
    Ok(())
}

/// Restore a list of targets, logging each and surfacing errors with context.
pub fn restore_targets(worker: &Worker, targets: &[PathBuf]) -> Result<()> {
    // Optional: run via switchyard facade when feature is enabled
    #[cfg(feature = "switchyard")]
    {
        return restore_targets_via_switchyard(worker, targets);
    }

    let mut pb = progress::new_bar(targets.len() as u64);
    let _quiet_guard = if pb.is_some() {
        Some(progress::enable_symlink_quiet())
    } else {
        None
    };
    let total = targets.len().max(1);
    for (idx, target) in targets.iter().enumerate() {
        tracing::trace!(step = "restore_item", target = %target.display());
        if pb.is_none() {
            tracing::info!(
                "[disable] Restoring {} (if backup exists)",
                target.display()
            );
        }
        // Structured audit: restore_started
        let _ = audit_event_fields(
            "symlink",
            "restore_started",
            "begin",
            &AuditFields {
                target: Some(target.display().to_string()),
                ..Default::default()
            },
        );
        // Emit host progress protocol line for v1 host bar
        if let Some(name) = target.file_name().and_then(|s| s.to_str()) {
            progress::emit_host_pb(idx + 1, total, &format!("Restoring {}", name));
        } else {
            progress::emit_host_pb(idx + 1, total, "Restoring");
        }
        let t0 = Instant::now();
        if let Err(e) = worker.restore_file(target) {
            tracing::error!("❌ Failed to restore {}: {}", target.display(), e);
            progress::finish(pb.take());
            return Err(Error::ExecutionFailed(format!(
                "failed to restore {}: {}",
                target.display(),
                e
            )));
        }
        let elapsed_ms = t0.elapsed().as_millis() as u64;
        tracing::debug!("restore_done: {} ({} ms)", target.display(), elapsed_ms);
        // Structured audit: restore_done
        let _ = audit_event_fields(
            "symlink",
            "restore_done",
            "success",
            &AuditFields {
                target: Some(target.display().to_string()),
                duration_ms: Some(elapsed_ms),
                ..Default::default()
            },
        );
        // Update progress after a successful restore
        if let Some(name) = target.file_name().and_then(|s| s.to_str()) {
            progress::set_msg_and_inc(&pb, format!("Restoring {}", name));
        } else {
            progress::set_msg_and_inc(&pb, "Restoring");
        }
    }
    progress::finish(pb);
    Ok(())
}

#[cfg(feature = "switchyard")]
fn restore_targets_via_switchyard(worker: &Worker, targets: &[PathBuf]) -> Result<()> {
    let policy = Policy {
        allow_roots: vec![PathBuf::from("/usr")],
        forbid_paths: vec![],
        strict_ownership: worker.strict_ownership,
        force_untrusted_source: worker.force_override_untrusted,
        force_restore_best_effort: worker.force_restore_best_effort,
    };
    let sx = Switchyard::new(NullFacts, NullAudit, policy);
    let mut input = PlanInput::default();
    for t in targets {
        input.restore.push(RestoreRequest { target: t.clone() });
    }
    let plan = sx.plan(input);
    let pre = sx.preflight(&plan);
    if !pre.ok {
        return Err(Error::ExecutionFailed(format!(
            "preflight failed: {}",
            pre.stops.join(", ")
        )));
    }
    let mode = if worker.dry_run {
        ApplyMode::DryRun
    } else {
        ApplyMode::Commit
    };
    let rep = sx.apply(&plan, mode);
    if !rep.errors.is_empty() {
        return Err(Error::ExecutionFailed(format!(
            "apply errors: {}",
            rep.errors.join(", ")
        )));
    }
    Ok(())
}

/// Log a short summary of the first `max_items` applets to be linked.
pub fn log_applets_summary(prefix: &str, applets: &[(String, PathBuf)], max_items: usize) {
    tracing::info!(
        "Preparing to link {} applet(s) for {}",
        applets.len(),
        prefix
    );
    for (i, (filename, src)) in applets.iter().enumerate() {
        if i >= max_items {
            tracing::info!("  (…truncated)");
            break;
        }
        let target = resolve_usrbin(filename);
        tracing::info!("  [{}] {} -> {}", i + 1, src.display(), target.display());
    }
}

/// Verify a package is installed, emitting explicit logs.
pub fn verify_installed(worker: &Worker, package: &str) -> Result<()> {
    if worker.check_installed(package)? {
        tracing::info!("✅ Expected: '{}' installed, Received: present", package);
        Ok(())
    } else {
        tracing::error!("❌ Expected: '{}' installed, Received: absent", package);
        Err(Error::ExecutionFailed(format!(
            "package '{}' not installed after operation",
            package
        )))
    }
}

/// Verify a package is removed, emitting explicit logs.
pub fn verify_removed(worker: &Worker, package: &str) -> Result<()> {
    if worker.check_installed(package)? {
        tracing::error!(
            "❌ Expected: '{}' absent after removal, Received: present",
            package
        );
        Err(Error::ExecutionFailed(format!(
            "package '{}' still installed after removal",
            package
        )))
    } else {
        tracing::info!(
            "✅ Expected: '{}' absent after removal, Received: absent",
            package
        );
        Ok(())
    }
}
