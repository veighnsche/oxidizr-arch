use crate::logging::audit_op;
use crate::ui::progress::symlink_info_enabled;
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Generate backup path for a target file (delegates to switchyard helper)
pub fn backup_path(target: &Path) -> PathBuf {
    switchyard::symlink::backup_path(target)
}

/// Validate path to prevent directory traversal attacks (delegates to switchyard helper)
pub fn is_safe_path(path: &Path) -> bool {
    switchyard::symlink::is_safe_path(path)
}

/// Atomically replace a file with a symlink, creating a backup. Delegates mechanism to switchyard and preserves product logging.
pub fn replace_file_with_symlink(source: &Path, target: &Path, dry_run: bool) -> Result<()> {
    if source == target {
        if symlink_info_enabled() {
            tracing::info!(
                "Source and target are the same ({}), skipping symlink.",
                source.display()
            );
        }
        return Ok(());
    }
    if !is_safe_path(source) || !is_safe_path(target) {
        return Err(crate::Error::ExecutionFailed(
            "Invalid path: contains directory traversal".into(),
        ));
    }

    // Log pre-state for observability parity (best-effort)
    let metadata = fs::symlink_metadata(target);
    let existed = metadata.is_ok();
    let is_symlink = metadata
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let current_dest = if is_symlink {
        fs::read_link(target).ok()
    } else {
        None
    };
    if symlink_info_enabled() {
        tracing::info!(
            "replace_file_with_symlink pre-state: target={}, existed={}, is_symlink={}, current_dest={}",
            target.display(), existed, is_symlink,
            current_dest.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<none>".into())
        );
    }

    if dry_run {
        if symlink_info_enabled() {
            tracing::info!(
                "[dry-run] would ensure symlink {} -> {} (updating/replacing as needed)",
                source.display(),
                target.display()
            );
        }
        return Ok(());
    }

    // Delegate to switchyard mechanism
    switchyard::symlink::replace_file_with_symlink(source, target, false)
        .map_err(crate::Error::Io)?;

    if symlink_info_enabled() {
        tracing::info!(
            "Symlink ensured: {} -> {}",
            target.display(),
            source.display()
        );
    }
    let _ = audit_op(
        "CREATE_SYMLINK",
        &format!("{} -> {}", target.display(), source.display()),
        true,
    );
    Ok(())
}

/// Restore a file from its backup. Delegates mechanism to switchyard and preserves product logging.
pub fn restore_file(target: &Path, dry_run: bool, force_best_effort: bool) -> Result<()> {
    let backup = backup_path(target);
    if backup.exists() {
        if dry_run {
            if symlink_info_enabled() {
                tracing::info!(
                    "[dry-run] would restore {} from {}",
                    target.display(),
                    backup.display()
                );
            }
            return Ok(());
        }
        switchyard::symlink::restore_file(target, false, force_best_effort)
            .map_err(crate::Error::Io)?;
        let _ = audit_op("RESTORE_FILE", &format!("{}", target.display()), true);
    } else {
        if force_best_effort {
            tracing::warn!("No backup for {}, leaving as-is", target.display());
        } else {
            return Err(crate::Error::RestoreBackupMissing(
                target.display().to_string(),
            ));
        }
    }
    Ok(())
}
