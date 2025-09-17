use std::path::Path;
use std::process::Command;

/// Detect whether SELinux is enabled on the target root.
/// Heuristic:
/// - If /sys/fs/selinux exists under the target root AND getenforce exists and
///   returns Enforcing or Permissive, treat as enabled.
/// - Otherwise, treat as disabled.
pub fn selinux_enabled(root: &Path) -> bool {
    // Check mountpoint presence inside root
    let selinux_fs = root.join("sys/fs/selinux");
    if !selinux_fs.exists() {
        return false;
    }
    // getenforce runs against the live kernel namespace; only meaningful when root is "/"
    if root != Path::new("/") {
        // We cannot reliably query kernel enforcing state for an alternate root
        // Consider SELinux disabled for gating purposes on non-live roots
        return false;
    }
    // Try to run getenforce
    if let Ok(out) = Command::new("getenforce").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
            return s.contains("enforcing") || s.contains("permissive");
        }
    }
    false
}
