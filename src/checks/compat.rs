use crate::error::{Error, Result};

/// Arch-family distros supported by default (without skip flag)
pub const SUPPORTED_DISTROS: [&str; 4] = ["arch", "endeavouros", "cachyos", "manjaro"];

/// Distribution metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distribution {
    pub id: String,
    pub id_like: String,
    pub release: String,
}

/// Check if a distro ID is in the supported set
pub fn is_supported_distro(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    SUPPORTED_DISTROS.contains(&id.as_str())
}

/// Validate distro compatibility, optionally allowing override
pub fn check_compatibility(distro: &Distribution, skip_check: bool) -> Result<()> {
    if skip_check {
        return Ok(());
    }

    if !is_supported_distro(&distro.id) {
        return Err(Error::Incompatible(format!(
            "Unsupported distro '{}'. Supported: {:?}. Pass --skip-compatibility-check to override.",
            distro.id,
            SUPPORTED_DISTROS
        )));
    }

    Ok(())
}
