use std::path::Path;

#[cfg(unix)]
pub fn sudo_guard(root: &Path, bin: &std::path::Path) -> Result<(), String> {
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    let md = fs::symlink_metadata(bin)
        .map_err(|e| format!("sudo guard: cannot stat {}: {}", bin.display(), e))?;
    let mode = md.mode();
    let uid = md.uid();
    let gid = md.gid();
    let is_setuid = (mode & 0o4000) != 0;
    let is_exec = (mode & 0o111) != 0;
    if root == Path::new("/") {
        // Live root: require strict root:root 4755
        if uid != 0 || gid != 0 || !is_setuid || !is_exec {
            return Err(format!(
                "replacement sudo is not setuid root (uid={}, gid={}, mode={:o}); ensure correct packaging/permissions",
                uid,
                gid,
                mode & 0o7777
            ));
        }
    } else {
        // Non-live root (hermetic tests): require setuid+exec at minimum
        if !is_setuid || !is_exec {
            return Err(format!(
                "replacement sudo is not setuid/exec (mode={:o}); set chmod 4755 in test artifact",
                mode & 0o7777
            ));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn sudo_guard(_root: &Path, _bin: &std::path::Path) -> Result<(), String> {
    Ok(())
}
