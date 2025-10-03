//! Extension plugin system for statements crate.
//!
//! This module provides a plugin system for extending the statements engine with custom
//! analysis capabilities. Extensions can process models and results to provide additional
//! insights, validations, or transformations.
//!
//! # Examples
//!
//! ```rust,ignore
//! use finstack_statements::extensions::{Extension, ExtensionRegistry, ExtensionContext, ExtensionResult};
//!
//! // Create an extension
//! struct MyExtension;
//!
//! impl Extension for MyExtension {
//!     fn metadata(&self) -> ExtensionMetadata {
//!         ExtensionMetadata {
//!             name: "my_extension".into(),
//!             version: "0.1.0".into(),
//!             description: Some("My custom extension".into()),
//!             author: None,
//!         }
//!     }
//!
//!     fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
//!         // Process the model and results
//!         Ok(ExtensionResult::success("Analysis complete"))
//!     }
//! }
//!
//! // Register and execute
//! let mut registry = ExtensionRegistry::new();
//! registry.register(Box::new(MyExtension));
//! ```

mod corkscrew;
mod plugin;
mod registry;
mod scorecards;

pub use corkscrew::CorkscrewExtension;
pub use plugin::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionStatus,
};
pub use registry::ExtensionRegistry;
pub use scorecards::CreditScorecardExtension;
