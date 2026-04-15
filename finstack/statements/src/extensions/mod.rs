//! Extension plugin system for the statements engine.
//!
//! The [`Extension`] trait and [`ExtensionRegistry`] are deprecated and
//! scheduled for removal in v0.5. Prefer the inherent methods on the
//! concrete analytics extension structs instead.

#![allow(deprecated)]

mod plugin;
mod registry;

pub use plugin::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus,
};
pub use registry::ExtensionRegistry;
