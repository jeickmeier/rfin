use finstack_core::config::FinstackConfig;
use finstack_core::Result;

use crate::registry::{
    margin_registry_from_config as resolve_registry, MarginRegistry, MARGIN_REGISTRY_EXTENSION_KEY,
};

/// Access the margin registry resolved from a `FinstackConfig` (applies overrides if present).
pub fn margin_registry_from_config(cfg: &FinstackConfig) -> Result<MarginRegistry> {
    resolve_registry(cfg)
}

/// Extension key for margin registry overrides.
pub const MARGIN_REGISTRY_EXTENSION: &str = MARGIN_REGISTRY_EXTENSION_KEY;
