//! Evaluator for financial models.
//!
//! The evaluator is responsible for:
//! - Building dependency graphs (DAG)
//! - Resolving evaluation order (topological sort)
//! - Applying precedence rules (Value > Forecast > Formula)
//! - Evaluating formulas for each period
//! - Handling where clause masking
//!
//! ## Where To Start
//!
//! - Use [`crate::evaluator::Evaluator`] for standard model evaluation.
//! - Use [`crate::evaluator::EvaluatorWithContext`] when you need to reuse configured runtime
//!   state across evaluations.
//! - Use [`crate::evaluator::StatementResult`] as the canonical output envelope for downstream
//!   analysis, reporting, and exports.
//! - Use the `to_polars_*` helpers only when the `dataframes` feature is
//!   enabled and you need tabular export.
//!
//! ## Conventions
//!
//! - Node precedence is `Value > Forecast > Formula`.
//! - Result values are stored as scalar `f64` outputs, with optional
//!   `NodeValueType` metadata preserving monetary-vs-scalar interpretation.
//! - Capital-structure outputs in [`crate::evaluator::StatementResult`] follow reporting-currency
//!   semantics when FX conversion is available; otherwise multi-currency totals
//!   may remain unavailable.

mod capital_structure_runtime;
mod cashflow_export;
mod context;
mod dag;
mod engine;
#[cfg(feature = "dataframes")]
mod export;
mod forecast_eval;
pub mod formula;
mod formula_aggregates;
pub(crate) mod formula_helpers;
pub(crate) mod monte_carlo;
mod precedence;
mod results;

pub use cashflow_export::{node_to_dated_schedule, PeriodDateConvention};
pub use context::EvaluationContext;
pub use dag::{evaluate_order, DependencyGraph};
pub use engine::{Evaluator, EvaluatorWithContext, PreparedEvaluation};
pub use monte_carlo::{MonteCarloConfig, MonteCarloResults, PercentileSeries};
pub use precedence::{resolve_node_value, NodeValueSource};
pub use results::{EvalWarning, NumericMode, ResultsMeta, StatementResult};

// Re-export DataFrame functionality when polars feature is enabled
#[cfg(feature = "dataframes")]
pub use export::{to_polars_long, to_polars_long_filtered, to_polars_wide};
