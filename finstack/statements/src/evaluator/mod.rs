//! Evaluator for financial models.
//!
//! The evaluator is responsible for:
//! - Building dependency graphs (DAG)
//! - Resolving evaluation order (topological sort)
//! - Applying precedence rules (Value > Forecast > Formula)
//! - Evaluating formulas for each period
//! - Handling where clause masking

mod context;
mod dag;
mod engine;
mod forecast_eval;
mod formula;
mod precedence;
mod results;

pub use context::StatementContext;
pub use engine::Evaluator;
pub use results::{Results, ResultsMeta};
pub use dag::{evaluate_order, DependencyGraph};
pub use precedence::{resolve_node_value, NodeValueSource};
