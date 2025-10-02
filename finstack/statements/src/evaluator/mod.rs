//! Evaluator for financial models.
//!
//! The evaluator is responsible for:
//! - Building dependency graphs (DAG)
//! - Resolving evaluation order (topological sort)
//! - Applying precedence rules (Value > Forecast > Formula)
//! - Evaluating formulas for each period
//! - Handling where clause masking

mod context;
mod core;
mod dag;
mod precedence;

pub use context::StatementContext;
pub use core::{Evaluator, Results, ResultsMeta};
pub use dag::{DependencyGraph, evaluate_order};
pub use precedence::{resolve_node_value, NodeValueSource};

