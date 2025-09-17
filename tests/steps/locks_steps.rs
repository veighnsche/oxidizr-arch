use cucumber::given;
use fs2::FileExt;
use std::fs::OpenOptions;

use crate::world::World;

#[given(regex = r"^a pacman db lock is held$")]
pub async fn pacman_db_lock_held(world: &mut World) {
    let lock_path = world.under_root("/var/lib/pacman/db.lck");
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let f = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    f.lock_exclusive().unwrap();
    world.pacman_lock = Some(f);
}
