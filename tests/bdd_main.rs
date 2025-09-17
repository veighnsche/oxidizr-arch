#[cfg(not(feature = "bdd"))]
fn main() {}

#[cfg(feature = "bdd")]
#[path = "world.rs"]
mod world;
#[cfg(feature = "bdd")]
#[path = "steps/mod.rs"]
mod steps;

#[cfg(feature = "bdd")]
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    use cucumber::World as _;
    use std::path::PathBuf;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let features_env = std::env::var("OXIDIZR_ARCH_BDD_FEATURE_PATH").ok();
    let features = if let Some(p) = features_env {
        let pb = PathBuf::from(p);
        if pb.is_absolute() {
            pb
        } else {
            root.join(pb)
        }
    } else {
        root.join("tests/features")
    };

    world::World::cucumber()
        .fail_on_skipped()
        .run_and_exit(features)
        .await;
}
