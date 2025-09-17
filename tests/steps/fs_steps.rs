use cucumber::then;
use std::fs;

use crate::world::World;

#[then(regex = r"^`(/.+)` is a regular file with content `(.+)`$")]
pub async fn path_is_regular_with_content(world: &mut World, path: String, content: String) {
    let abs = world.under_root(&path);
    let md = fs::symlink_metadata(&abs).expect("stat path");
    assert!(md.file_type().is_file(), "expected regular file at {}", abs.display());
    let s = fs::read_to_string(&abs).expect("read file");
    assert!(s.starts_with(&content), "expected file content to start with {:?}, got {:?}", content, s);
}

#[then(regex = r"^`(/.+)` is a regular file$")]
pub async fn path_is_regular(world: &mut World, path: String) {
    let abs = world.under_root(&path);
    let md = fs::symlink_metadata(&abs).expect("stat path");
    assert!(md.file_type().is_file(), "expected regular file at {}", abs.display());
}

#[then(regex = r"^`(/.+)` is a symlink$")]
pub async fn path_is_symlink(world: &mut World, path: String) {
    let abs = world.under_root(&path);
    let md = fs::symlink_metadata(&abs).expect("stat path");
    assert!(md.file_type().is_symlink(), "expected symlink at {}", abs.display());
}

#[then(regex = r"^`(/.+)` is a symlink to the replacement$")]
pub async fn path_is_symlink_to_replacement(world: &mut World, path: String) {
    let abs = world.under_root(&path);
    let md = fs::symlink_metadata(&abs).expect("stat path");
    assert!(md.file_type().is_symlink(), "expected symlink at {}", abs.display());
    let dest = fs::read_link(&abs).expect("readlink");
    let expected_rel = world.artifact_path.as_ref().expect("artifact path set by setup").clone();
    let expected_abs = world.under_root(expected_rel);
    assert_eq!(dest, expected_abs, "expected link dest to be replacement: {} -> {}", abs.display(), expected_abs.display());
}
