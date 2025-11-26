//! Financial statement modeling templates.
//!
//! This module provides high-level structural templates that automate the creation of
//! connected nodes for common financial modeling scenarios like roll-forwards and
//! vintage analysis.
//!
//! # Build-time vs Runtime
//!
//! These templates are **build-time** helpers that extend [`ModelBuilder`](crate::builder::ModelBuilder)
//! to create properly connected node structures. For **runtime validation** of these
//! structures after evaluation, see [`CorkscrewExtension`](crate::extensions::CorkscrewExtension).
//!
//! | Template | Build-time | Runtime Validation |
//! |----------|------------|-------------------|
//! | Roll-forward | [`TemplatesExtension::add_roll_forward`] | [`CorkscrewExtension`](crate::extensions::CorkscrewExtension) |
//! | Vintage | [`VintageExtension::add_vintage_buildup`] | N/A |
//!
//! # Example
//!
//! ```rust,ignore
//! use finstack_statements::prelude::*;
//! use finstack_statements::templates::TemplatesExtension;
//!
//! let model = ModelBuilder::new("demo")
//!     .periods("2025Q1..Q4", None)?
//!     .value("additions", &values)
//!     .value("disposals", &values)
//!     .add_roll_forward("inventory", &["additions"], &["disposals"])?
//!     .build()?;
//! ```

pub mod builder;
pub mod roll_forward;
pub mod vintage;

pub use builder::{TemplatesExtension, VintageExtension};
