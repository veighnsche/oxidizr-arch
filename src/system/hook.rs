use crate::logging::{audit_event_fields, AuditFields};
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};

const HOOK_DIR: &str = "/usr/share/libalpm/hooks";
const HOOK_NAME: &str = "oxidizr-arch-relink.hook";

pub fn hook_body() -> String {
    // Keep trigger broad; relink-managed will read state and only relink needed targets.
    // Using PostTransaction ensures we act after packages have written files.
    let exec = "/usr/bin/oxidizr-arch relink-managed --assume-yes --no-update --no-progress";
    format!(
        "[Trigger]\nType = Package\nOperation = Install\nOperation = Upgrade\nOperation = Remove\nTarget = coreutils\nTarget = findutils\nTarget = sudo\nTarget = sudo-rs\n\n[Action]\nDescription = Relink oxidizr-arch managed symlinks\nWhen = PostTransaction\nExec = {}\n",
        exec
    )
}

/// Compute the absolute path where the pacman hook will be installed.
pub fn hook_path() -> PathBuf {
    Path::new(HOOK_DIR).join(HOOK_NAME)
}

pub fn install_pacman_hook() -> Result<PathBuf> {
    let dir = Path::new(HOOK_DIR);
    let path = dir.join(HOOK_NAME);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| crate::Error::HookInstallError(e.to_string()))?;
    }
    let body = hook_body();
    fs::write(&path, body.as_bytes()).map_err(|e| crate::Error::HookInstallError(e.to_string()))?;
    let _ = audit_event_fields(
        "hook",
        "install_pacman_hook",
        "success",
        &AuditFields {
            artifacts: Some(vec![path.display().to_string()]),
            ..Default::default()
        },
    );
    Ok(path)
}
