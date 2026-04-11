//! Helper functions for portfolio optimization use cases.
//!
//! These helpers live in the core crate so that bindings (Python, WASM)
//! only need to perform type conversions and can pass through directly
//! to Rust logic.

use super::{
    Constraint, DefaultLpOptimizer, MissingMetricPolicy, Objective, PortfolioOptimizationProblem,
    WeightingScheme,
};
use crate::error::Result;
use crate::portfolio::{Portfolio, PortfolioSpec};
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::result::TradeSpec;

// ---------------------------------------------------------------------------
// General-purpose optimization spec (JSON-friendly)
// ---------------------------------------------------------------------------

/// JSON-serializable specification for a portfolio optimization problem.
///
/// This type bridges the gap between the JSON-first binding pattern and the
/// internal [`PortfolioOptimizationProblem`] which holds a live `Portfolio`.
/// Bindings deserialize this spec, build the `Portfolio` from the embedded
/// [`PortfolioSpec`], and then run the optimizer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioOptimizationSpec {
    /// Portfolio specification (same format as `value_portfolio`).
    pub portfolio: PortfolioSpec,
    /// Optimization objective.
    pub objective: Objective,
    /// Constraints on the optimized portfolio.
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// How weights are defined.
    #[serde(default = "default_weighting")]
    pub weighting: WeightingScheme,
    /// Policy for handling positions missing required metrics.
    #[serde(default)]
    pub missing_metric_policy: MissingMetricPolicy,
    /// Optional label for auditability.
    #[serde(default)]
    pub label: Option<String>,
}

fn default_weighting() -> WeightingScheme {
    WeightingScheme::ValueWeight
}

/// JSON-serializable result of a portfolio optimization.
///
/// Extracts the key fields from [`super::PortfolioOptimizationResult`] into a
/// serde-friendly structure suitable for Python/WASM round-trips.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioOptimizationResultJson {
    /// Optimization status.
    pub status: super::OptimizationStatus,
    /// Human-readable status string.
    pub status_label: String,
    /// Whether the result is feasible (usable).
    pub is_feasible: bool,
    /// Objective value at the solution.
    pub objective_value: f64,
    /// Total one-way turnover.
    pub turnover: f64,
    /// Optimal weights per position.
    pub optimal_weights: IndexMap<PositionId, f64>,
    /// Current weights per position (pre-trade).
    pub current_weights: IndexMap<PositionId, f64>,
    /// Weight deltas (`optimal - current`).
    pub weight_deltas: IndexMap<PositionId, f64>,
    /// Implied target quantities.
    pub implied_quantities: IndexMap<PositionId, f64>,
    /// Evaluated portfolio-level metrics at the solution.
    pub metric_values: IndexMap<String, f64>,
    /// Trade list (sorted by absolute delta, largest first).
    pub trades: Vec<TradeSpec>,
    /// Shadow prices / dual values for constraints.
    pub dual_values: IndexMap<String, f64>,
    /// Constraint slack values.
    pub constraint_slacks: IndexMap<String, f64>,
    /// Binding constraints (slack near zero).
    pub binding_constraints: Vec<String>,
    /// Optional label from the problem.
    pub label: Option<String>,
}

/// Run portfolio optimization from a JSON-friendly spec.
///
/// Builds the `Portfolio` from the embedded `PortfolioSpec`, constructs the
/// optimization problem, and returns a serializable result.
pub fn optimize_from_spec(
    spec: &PortfolioOptimizationSpec,
    market: &MarketContext,
    config: &FinstackConfig,
) -> Result<PortfolioOptimizationResultJson> {
    let portfolio = Portfolio::from_spec(spec.portfolio.clone())?;

    let mut problem = PortfolioOptimizationProblem::new(portfolio, spec.objective.clone());
    problem.weighting = spec.weighting;
    problem.missing_metric_policy = spec.missing_metric_policy;
    problem.label = spec.label.clone();
    problem = problem.with_constraints(spec.constraints.clone());

    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer.optimize(&problem, market, config)?;

    let binding = result
        .binding_constraints()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    Ok(PortfolioOptimizationResultJson {
        status_label: format!("{:?}", result.status),
        is_feasible: result.status.is_feasible(),
        status: result.status.clone(),
        objective_value: result.objective_value,
        turnover: result.turnover(),
        optimal_weights: result.optimal_weights.clone(),
        current_weights: result.current_weights.clone(),
        weight_deltas: result.weight_deltas.clone(),
        implied_quantities: result.implied_quantities.clone(),
        metric_values: result.metric_values.clone(),
        trades: result.to_trade_list(),
        dual_values: result.dual_values.clone(),
        constraint_slacks: result.constraint_slacks.clone(),
        binding_constraints: binding,
        label: result.problem.label.clone(),
    })
}
