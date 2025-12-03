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
#[cfg(feature = "dataframes")]
mod export;
mod forecast_eval;
pub(crate) mod formula;
mod precedence;
mod results;

pub use context::EvaluationContext;
pub use dag::{evaluate_order, DependencyGraph};
pub use engine::{Evaluator, EvaluatorWithContext};
pub use precedence::{resolve_node_value, NodeValueSource};
pub use results::{EvalWarning, NumericMode, Results, ResultsMeta};

// Re-export DataFrame functionality when polars feature is enabled
#[cfg(feature = "dataframes")]
pub use export::{to_polars_long, to_polars_long_filtered, to_polars_wide};
