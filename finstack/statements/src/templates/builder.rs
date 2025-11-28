//! Extension trait for ModelBuilder to support templates.

use super::roll_forward;
use super::vintage;
use crate::builder::{ModelBuilder, Ready};
use crate::error::Result;

/// Extension methods for `ModelBuilder` to support high-level modeling templates.
pub trait TemplatesExtension<State> {
    /// Add a roll-forward structure (Beginning + Increases - Decreases = Ending).
    fn add_roll_forward(
        self,
        name: &str,
        increases: &[&str],
        decreases: &[&str],
    ) -> Result<ModelBuilder<State>>;
}

impl<State> TemplatesExtension<State> for ModelBuilder<State> {
    fn add_roll_forward(
        self,
        name: &str,
        increases: &[&str],
        decreases: &[&str],
    ) -> Result<ModelBuilder<State>> {
        roll_forward::add_roll_forward(self, name, increases, decreases)
    }
}

/// Extension methods for `ModelBuilder<Ready>` (requires periods).
pub trait VintageExtension {
    /// Add a vintage buildup (cohort analysis) structure.
    fn add_vintage_buildup(
        self,
        name: &str,
        new_volume_node: &str,
        decay_curve: &[f64],
    ) -> Result<ModelBuilder<Ready>>;
}

impl VintageExtension for ModelBuilder<Ready> {
    fn add_vintage_buildup(
        self,
        name: &str,
        new_volume_node: &str,
        decay_curve: &[f64],
    ) -> Result<ModelBuilder<Ready>> {
        vintage::add_vintage_buildup(self, name, new_volume_node, decay_curve)
    }
}
