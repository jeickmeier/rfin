//! Extension plugin system for the statements engine.
//!
//! This module provides the [`Extension`] trait and [`ExtensionRegistry`] for
//! building custom analysis and validation plugins.
//!
//! For built-in extensions (corkscrew, credit scorecard), enable the `analytics`
//! feature or depend on `finstack-statements-analytics` directly.

mod plugin;
mod registry;

pub use plugin::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus,
};
pub use registry::ExtensionRegistry;
