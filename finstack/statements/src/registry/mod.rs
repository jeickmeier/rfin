//! Dynamic metric registry system.
//!
//! The registry allows loading reusable metric definitions from JSON files,
//! enabling analysts to define standard financial metrics without recompiling.
//!
//! ## Features
//!
//! - **JSON-based**: Metrics defined in JSON files
//! - **Namespaces**: Organize metrics by namespace (e.g., `fin.*`, `custom.*`)
//! - **Collision detection**: Prevent duplicate metric IDs
//! - **Built-in metrics**: Standard financial metrics included
//!
//! ## Example
//!
//! ```rust
//! use finstack_statements::registry::Registry;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut registry = Registry::new();
//!
//! // Load built-in metrics
//! registry.load_builtins()?;
//!
//! // Get a metric
//! let metric = registry.get("fin.gross_margin")?;
//! println!("Formula: {}", metric.definition.formula);
//! # Ok(())
//! # }
//! ```

mod builtins;
mod dynamic;
mod schema;
mod validation;

pub use dynamic::Registry;
pub use schema::{MetricDefinition, MetricRegistry, UnitType};
pub use validation::validate_metric_definition;
