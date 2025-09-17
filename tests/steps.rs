#[cfg(feature = "bdd")]
use cucumber::then;
#[cfg(feature = "bdd")]
use crate::world::World;

#[cfg(feature = "bdd")]
#[then("it works")]
async fn it_works(_world: &mut World) {}
