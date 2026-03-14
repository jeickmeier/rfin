//! Helper functions for portfolio optimization use cases.
//!
//! These helpers live in the core crate so that bindings (Python, WASM)
//! only need to perform type conversions and can pass through directly
//! to Rust logic.

use super::{
    Constraint, DefaultLpOptimizer, MetricExpr, MissingMetricPolicy, Objective, PerPositionMetric,
    PortfolioOptimizationProblem,
};
use crate::error::Result;
use crate::portfolio::Portfolio;
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::metrics::MetricId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Result of the max-yield optimization helper.
///
/// This mirrors the data returned by the Python helper while keeping the
/// binding layer focused on serialization only.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaxYieldWithCccLimitResult {
    /// Label propagated from the optimization problem.
    pub label: Option<String>,
    /// Optimization status.
    pub status: crate::optimization::OptimizationStatus,
    /// Human-readable status string (mirrors `Debug` formatting).
    pub status_label: String,
    /// Objective value at the solution.
    pub objective_value: f64,
    /// Aggregate weight of positions tagged `rating="CCC"`.
    pub ccc_weight: f64,
    /// Optimal weights per position.
    pub optimal_weights: IndexMap<PositionId, f64>,
    /// Current weights per position (pre-trade).
    pub current_weights: IndexMap<PositionId, f64>,
    /// Weight deltas (`optimal - current`).
    pub weight_deltas: IndexMap<PositionId, f64>,
}

/// Optimize a bond portfolio to maximize value‑weighted YTM with a CCC exposure limit.
///
/// This helper mirrors the Python binding helper and returns a serializable
/// struct so bindings only perform type conversions.
pub fn optimize_max_yield_with_ccc_limit(
    portfolio: &Portfolio,
    market_context: &MarketContext,
    config: &FinstackConfig,
    ccc_limit: f64,
    strict_risk: bool,
) -> Result<MaxYieldWithCccLimitResult> {
    // Objective: maximize value‑weighted average yield (YTM).
    let objective = Objective::Maximize(MetricExpr::ValueWeightedAverage {
        metric: PerPositionMetric::Metric(MetricId::Ytm),
    });

    let mut problem = PortfolioOptimizationProblem::new(portfolio.clone(), objective);
    problem.weighting = super::WeightingScheme::ValueWeight;
    problem.missing_metric_policy = if strict_risk {
        MissingMetricPolicy::Strict
    } else {
        MissingMetricPolicy::Zero
    };
    problem.label = Some("max_yield_with_ccc_limit".to_string());

    // CCC exposure constraint.
    problem = problem.with_constraint(Constraint::TagExposureLimit {
        label: Some("ccc_limit".to_string()),
        tag_key: "rating".to_string(),
        tag_value: "CCC".to_string(),
        max_share: ccc_limit,
    });

    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer.optimize(&problem, market_context, config)?;

    // Compute CCC exposure from optimal weights and tags.
    let mut ccc_weight = 0.0_f64;
    let portfolio_ref = &result.problem.portfolio;
    for (pos_id, &w) in &result.optimal_weights {
        if let Some(position) = portfolio_ref.get_position(pos_id.as_str()) {
            if position.tags.get("rating").map(String::as_str) == Some("CCC") {
                ccc_weight += w;
            }
        }
    }

    Ok(MaxYieldWithCccLimitResult {
        label: result.problem.label.clone(),
        status_label: format!("{:?}", result.status),
        status: result.status.clone(),
        objective_value: result.objective_value,
        ccc_weight,
        optimal_weights: result.optimal_weights.clone(),
        current_weights: result.current_weights.clone(),
        weight_deltas: result.weight_deltas.clone(),
    })
}
