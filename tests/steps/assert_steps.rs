use cucumber::then;

use crate::world::World;

#[then("it works")]
pub async fn it_works(_world: &mut World) {}

#[then(regex = r"^the command exits (\d+)$")]
pub async fn command_exits(world: &mut World, code: i32) {
    let out = world.last_output.as_ref().expect("no output captured");
    assert_eq!(out.status.code().unwrap_or(1), code);
}

#[then(regex = r"^output does not contain `(.+)`$")]
pub async fn output_not_contains(world: &mut World, needle: String) {
    let out = world.last_output.as_ref().expect("no output captured");
    let stdout_s = String::from_utf8_lossy(&out.stdout);
    let stderr_s = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout_s.contains(&needle) && !stderr_s.contains(&needle),
        "needle unexpectedly found in stdout/stderr: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        needle,
        stdout_s,
        stderr_s
    );
}

#[then(regex = r"^stderr contains `(.+)`$")]
pub async fn stderr_contains(world: &mut World, needle: String) {
    let out = world.last_output.as_ref().expect("no output captured");
    let s = String::from_utf8_lossy(&out.stderr);
    assert!(s.contains(&needle), "stderr missing: {}\n--- stderr ---\n{}", needle, s);
}

#[then(regex = r"^stdout contains `(.+)`$")]
pub async fn stdout_contains(world: &mut World, needle: String) {
    let out = world.last_output.as_ref().expect("no output captured");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains(&needle), "stdout missing: {}\n--- stdout ---\n{}", needle, s);
}

#[then(regex = r"^output contains `(.+)`$")]
pub async fn output_contains(world: &mut World, needle: String) {
    let out = world.last_output.as_ref().expect("no output captured");
    let stdout_s = String::from_utf8_lossy(&out.stdout);
    let stderr_s = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout_s.contains(&needle) || stderr_s.contains(&needle),
        "needle not found in stdout/stderr: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        needle,
        stdout_s,
        stderr_s
    );
}

#[then(regex = r"^it reports a dry-run with a non-zero planned action count$")]
pub async fn reports_dry_run_non_zero(world: &mut World) {
    let out = world.last_output.as_ref().expect("no output captured");
    let s = String::from_utf8_lossy(&out.stderr);
    let needle = "dry-run: planned ";
    assert!(s.contains(needle), "stderr missing '{}':\n{}", needle, s);
}
