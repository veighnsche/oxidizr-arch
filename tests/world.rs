#[cfg(feature = "bdd")]
use cucumber::{World, WorldInit};

#[cfg(feature = "bdd")]
#[derive(Debug, Default, WorldInit)]
pub struct World {
    pub root: Option<tempfile::TempDir>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(feature = "bdd")]
#[async_trait::async_trait]
impl World for World {
    type Error = std::convert::Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}
