use cucumber::given;
use std::path::PathBuf;

use crate::world::World;

#[given(regex = r"^a staging root at .*$")]
pub async fn staging_root_at(world: &mut World) {
    // Prepare basic directories
    world.ensure_dir("/usr/bin");
    world.ensure_dir("/var/lock");
}

#[given(regex = r#"^a verified replacement artifact lists applets \"([^"]+)\" for package \"(coreutils|findutils|sudo)\"$"#)]
pub async fn verified_artifact_lists_applets(world: &mut World, applets: String, pkg: String) {
    use std::io::Write as _;
    let rel = match pkg.as_str() {
        "coreutils" => std::path::PathBuf::from("/opt/uutils/uutils"),
        "findutils" => std::path::PathBuf::from("/opt/uutils-findutils/uutils-findutils"),
        "sudo" => std::path::PathBuf::from("/opt/sudo-rs/sudo-rs"),
        _ => unreachable!(),
    };
    let abs = world.under_root(&rel);
    if let Some(parent) = abs.parent() { let _ = std::fs::create_dir_all(parent); }
    let content = format!("#!/bin/sh\nif [ \"$1\" = \"--list\" ] || [ \"$1\" = \"--help\" ]; then\n  echo {}\n  exit 0\nfi\necho {}\n", shlex::quote(&applets), shlex::quote(&applets));
    {
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&abs).unwrap().permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&abs, perms);
    }
    world.artifact_path = Some(rel);
}

#[given(regex = r"^a regular file exists at `(/.+)` with content `(.+)`$")]
pub async fn create_regular_file_with_content(world: &mut World, path: String, content: String) {
    world.write_file(&path, content.as_bytes(), false);
}

#[given(regex = r"^a fakeroot with stock coreutils applets$")]
pub async fn fakeroot_with_stock_coreutils(world: &mut World) {
    world.ensure_dir("/usr/bin");
    world.write_file("/usr/bin/ls", b"gnu-ls", true);
    world.write_file("/usr/bin/cat", b"gnu-cat", true);
    world.write_file("/usr/bin/find", b"gnu-find", true);
    world.write_file("/usr/bin/xargs", b"gnu-xargs", true);
}

#[given(regex = r#"^a verified replacement artifact is available for package \"(coreutils|findutils|sudo)\"$"#)]
pub async fn verified_artifact_available(world: &mut World, pkg: String) {
    let (rel_path, contents): (PathBuf, &'static [u8]) = match pkg.as_str() {
        "coreutils" => (PathBuf::from("/opt/uutils/uutils"), b"uutils-binary"),
        "findutils" => (PathBuf::from("/opt/uutils-findutils/uutils-findutils"), b"uutils-findutils-binary"),
        "sudo" => (PathBuf::from("/opt/sudo-rs/sudo-rs"), b"sudo-rs-binary"),
        _ => unreachable!(),
    };
    let abs = world.under_root(&rel_path);
    if let Some(parent) = abs.parent() { let _ = std::fs::create_dir_all(parent); }
    std::fs::write(&abs, contents).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&abs).unwrap().permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&abs, perms);
    }
    world.artifact_path = Some(rel_path);
}

#[given(regex = r"^the sudo artifact has setuid 4755$")]
pub async fn sudo_artifact_setuid(world: &mut World) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let rel = world.artifact_path.as_ref().expect("artifact").clone();
        let abs = world.under_root(rel);
        let mut perms = std::fs::metadata(&abs).unwrap().permissions();
        perms.set_mode(0o4755);
        let _ = std::fs::set_permissions(&abs, perms);
    }
}
