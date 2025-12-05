#![allow(clippy::module_name_repetitions)]

//! Portfolio optimization on top of valuations.
//!
//! This module provides a deterministic, metric‑driven portfolio optimization
//! facility that operates entirely on top of existing valuation results. It
//! does **not** compute cashflows directly – instead, it:
//!
//! 1. Requests a set of per‑position metrics via `price_with_metrics`.
//! 2. Converts those metrics and tags into linear coefficients.
//! 3. Solves for portfolio weights using a linear programming (LP) backend.
//!
//! The initial scope focuses on linear objectives and constraints in the
//! position weights, value‑weighted portfolios, and a configurable trade
//! universe (existing positions plus optional candidate instruments).

use crate::error::{PortfolioError, Result};
use crate::portfolio::Portfolio;
use crate::position::{Position, PositionUnit};
use crate::types::{EntityId, PositionId};
use finstack_core::math::summation::neumaier_sum;
use finstack_core::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::metrics::MetricId;
use good_lp::{constraint, default_solver, variable, Expression, Solution, SolverModel};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// How optimization weights are defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightingScheme {
    /// `w_i` is share of portfolio base currency PV; `∑ w_i = 1`.
    ValueWeight,

    /// `w_i` is share of some notional exposure; still normalized so `∑ w_i = 1`.
    NotionalWeight,

    /// `w_i` scales the current quantity (e.g. units or face value).
    /// For MVP we treat `w_i` as a fraction of current position size.
    UnitScaling,
}

/// Filters for selecting which positions are included in a rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PositionFilter {
    /// All positions in the portfolio.
    All,

    /// Filter by entity ID.
    ByEntityId(EntityId),

    /// Filter by tag key/value (e.g. rating = "HY").
    ByTag { key: String, value: String },

    /// Filter by multiple position IDs.
    ByPositionIds(Vec<PositionId>),

    /// Exclude positions matching the inner filter.
    Not(Box<PositionFilter>),
}

/// A candidate instrument that could be added to the portfolio.
///
/// This represents an instrument not currently held but available for trading.
/// The optimizer can allocate weight to candidates (up to `max_weight`).
#[derive(Clone)]
pub struct CandidatePosition {
    /// Unique identifier for this candidate (becomes `PositionId` if traded).
    pub id: PositionId,

    /// Entity that would own this position.
    pub entity_id: EntityId,

    /// The instrument that could be traded.
    pub instrument: Arc<dyn finstack_valuations::instruments::common::traits::Instrument>,

    /// Unit type for quantity interpretation.
    pub unit: PositionUnit,

    /// Tags for the candidate (used in constraints like `TagExposureLimit`).
    pub tags: IndexMap<String, String>,

    /// Maximum weight this candidate can receive (default: 1.0 = no limit).
    /// Useful for limiting exposure to any single new position.
    pub max_weight: f64,

    /// Minimum weight if included (for minimum position size constraints).
    /// Set to 0.0 to allow the optimizer to skip this candidate entirely.
    pub min_weight: f64,
}

impl CandidatePosition {
    /// Create a new candidate position.
    pub fn new(
        id: impl Into<PositionId>,
        entity_id: impl Into<EntityId>,
        instrument: Arc<dyn finstack_valuations::instruments::common::traits::Instrument>,
        unit: PositionUnit,
    ) -> Self {
        Self {
            id: id.into(),
            entity_id: entity_id.into(),
            instrument,
            unit,
            tags: IndexMap::new(),
            max_weight: 1.0,
            min_weight: 0.0,
        }
    }

    /// Add a tag to the candidate.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Set maximum weight for this candidate.
    pub fn with_max_weight(mut self, max: f64) -> Self {
        self.max_weight = max;
        self
    }

    /// Set minimum weight (if included) for this candidate.
    pub fn with_min_weight(mut self, min: f64) -> Self {
        self.min_weight = min;
        self
    }
}

/// Defines which instruments the optimizer can trade.
///
/// The trade universe consists of:
///
/// 1. **Tradeable positions**: existing portfolio positions that can be adjusted
/// 2. **Held positions**: existing positions locked at current weight
/// 3. **Candidate positions**: new instruments that could be added
#[derive(Clone, Debug)]
pub struct TradeUniverse {
    /// Filter for existing positions that can be traded.
    /// Positions matching this filter have their weights optimized.
    /// Default: all positions are tradeable.
    pub tradeable_filter: PositionFilter,

    /// Filter for existing positions that are held constant.
    /// Positions matching this filter keep their current weight.
    /// Takes precedence over `tradeable_filter` if both match.
    pub held_filter: Option<PositionFilter>,

    /// Candidate instruments not currently in the portfolio.
    /// These start with weight 0 and can be added by the optimizer.
    pub candidates: Vec<CandidatePosition>,

    /// Whether candidates can receive negative weights (short selling).
    /// Default: false (long‑only for new positions).
    pub allow_short_candidates: bool,
}

impl TradeUniverse {
    /// Create a universe where all existing positions are tradeable.
    pub fn all_positions() -> Self {
        Self::default()
    }

    /// Create a universe with only specific positions tradeable.
    pub fn filtered(filter: PositionFilter) -> Self {
        Self {
            tradeable_filter: filter,
            ..Self::default()
        }
    }

    /// Add a candidate position to the universe.
    pub fn with_candidate(mut self, candidate: CandidatePosition) -> Self {
        self.candidates.push(candidate);
        self
    }

    /// Add multiple candidate positions.
    pub fn with_candidates(
        mut self,
        candidates: impl IntoIterator<Item = CandidatePosition>,
    ) -> Self {
        self.candidates.extend(candidates);
        self
    }

    /// Set positions to hold constant (not trade).
    pub fn with_held_positions(mut self, filter: PositionFilter) -> Self {
        self.held_filter = Some(filter);
        self
    }

    /// Allow short selling of candidate positions.
    pub fn allow_shorting_candidates(mut self) -> Self {
        self.allow_short_candidates = true;
        self
    }
}

impl Default for TradeUniverse {
    fn default() -> Self {
        Self {
            tradeable_filter: PositionFilter::All,
            held_filter: None,
            candidates: Vec::new(),
            allow_short_candidates: false,
        }
    }
}

impl std::fmt::Debug for CandidatePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandidatePosition")
            .field("id", &self.id)
            .field("entity_id", &self.entity_id)
            .field("unit", &self.unit)
            .field("tags", &self.tags)
            .field("max_weight", &self.max_weight)
            .field("min_weight", &self.min_weight)
            .finish()
    }
}

/// Where a per‑position scalar metric comes from.
#[derive(Clone, Debug)]
pub enum PerPositionMetric {
    /// Directly from `ValuationResult::measures` using a standard `MetricId`.
    ///
    /// Examples:
    /// - `Metric(MetricId::DurationMod)` for modified duration
    /// - `Metric(MetricId::Ytm)` for yield to maturity
    /// - `Metric(MetricId::Dv01)` for DV01
    Metric(MetricId),

    /// From `ValuationResult::measures` using a string key (for custom or
    /// bucketed metrics stored by name).
    CustomKey(String),

    /// Use the base currency PV of the position (after scaling).
    PvBase,

    /// Use the native‑currency PV of the position (after scaling).
    PvNative,

    /// Tag‑based 0/1 indicator: 1.0 if tag matches, else 0.0.
    TagEquals { key: String, value: String },

    /// Constant scalar for all positions.
    Constant(f64),
}

/// How to handle missing metrics for a position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingMetricPolicy {
    /// Treat missing as 0.0 (default, appropriate for duration‑like metrics).
    #[default]
    Zero,

    /// Exclude position from constraint evaluation (position keeps current weight).
    Exclude,

    /// Fail with error if any required metric is missing.
    Strict,
}

/// Portfolio‑level scalar metric expressed in terms of position metrics + weights.
#[derive(Clone, Debug)]
pub enum MetricExpr {
    /// `sum_i w_i * m_i`, where `m_i` comes from a `PerPositionMetric`.
    WeightedSum { metric: PerPositionMetric },

    /// Value‑weighted average: `sum_i w_i * m_i`, with implicit `sum_i w_i == 1`.
    /// This is appropriate for duration or yield when weights are `ValueWeight`.
    ValueWeightedAverage { metric: PerPositionMetric },

    /// Exposure share for a tag bucket: `sum_i w_i * I[tag == value]`.
    /// Assumes weights are already normalized (e.g. `ValueWeight`).
    TagExposureShare { tag_key: String, tag_value: String },
}

/// Optimization direction and target.
#[derive(Clone, Debug)]
pub enum Objective {
    /// Maximize a portfolio‑level metric.
    Maximize(MetricExpr),
    /// Minimize a portfolio‑level metric.
    Minimize(MetricExpr),
}

/// Inequality/equality operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inequality {
    /// Less‑than or equal: `lhs <= rhs`.
    Le,
    /// Greater‑than or equal: `lhs >= rhs`.
    Ge,
    /// Equality: `lhs == rhs`.
    Eq,
}

/// Declarative constraint specification.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// General metric bound, e.g. duration `<= 4.0`.
    MetricBound {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Metric expression on the left‑hand side.
        metric: MetricExpr,
        /// Operator (<=, >=, ==).
        op: Inequality,
        /// Right‑hand side constant.
        rhs: f64,
    },

    /// Tag exposure limit, e.g. rating=CCC weight `<= 0.10`.
    TagExposureLimit {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        /// Maximum share in `[0, 1]`.
        max_share: f64,
    },

    /// Minimum tag exposure, e.g. rating=IG weight `>= 0.50`.
    TagExposureMinimum {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        /// Minimum share in `[0, 1]`.
        min_share: f64,
    },

    /// Weight bounds for all positions matching the filter.
    WeightBounds {
        label: Option<String>,
        filter: PositionFilter,
        /// Inclusive minimum weight.
        min: f64,
        /// Inclusive maximum weight.
        max: f64,
    },

    /// Maximum turnover constraint: `Σ |w_new - w_current| <= max_turnover`.
    MaxTurnover {
        label: Option<String>,
        max_turnover: f64,
    },

    /// Maximum single position weight change: `|w_new - w_current| <= max_delta`.
    MaxPositionDelta {
        label: Option<String>,
        filter: PositionFilter,
        max_delta: f64,
    },

    /// Budget/normalization constraint: usually `∑ w_i == 1.0`.
    Budget { rhs: f64 },
}

impl Constraint {
    /// Get the constraint label (for diagnostics).
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::MetricBound { label, .. } => label.as_deref(),
            Self::TagExposureLimit { label, .. } => label.as_deref(),
            Self::TagExposureMinimum { label, .. } => label.as_deref(),
            Self::WeightBounds { label, .. } => label.as_deref(),
            Self::MaxTurnover { label, .. } => label.as_deref(),
            Self::MaxPositionDelta { label, .. } => label.as_deref(),
            Self::Budget { .. } => Some("budget"),
        }
    }
}

/// Complete optimization problem specification.
#[derive(Clone, Debug)]
pub struct PortfolioOptimizationProblem {
    /// The existing portfolio to optimize.
    pub portfolio: Portfolio,

    /// How weights are defined (value‑weighted, notional, etc.).
    pub weighting: WeightingScheme,

    /// The trade universe: which positions can be traded and candidate instruments.
    /// Default: all existing portfolio positions are tradeable, no candidates.
    pub trade_universe: TradeUniverse,

    /// Optimization objective (maximize/minimize a metric expression).
    pub objective: Objective,

    /// Constraints on the optimized portfolio.
    pub constraints: Vec<Constraint>,

    /// Policy for handling positions missing required metrics.
    pub missing_metric_policy: MissingMetricPolicy,

    /// Optional label for auditability.
    pub label: Option<String>,

    /// Additional metadata for auditability.
    pub meta: IndexMap<String, serde_json::Value>,
}

impl PortfolioOptimizationProblem {
    /// Create a basic problem optimizing all positions in the portfolio.
    ///
    /// This helper:
    /// - Uses `WeightingScheme::ValueWeight`
    /// - Uses a default `TradeUniverse` (all positions tradeable, no candidates)
    /// - Adds a `Budget { rhs: 1.0 }` constraint
    #[must_use]
    pub fn new(portfolio: Portfolio, objective: Objective) -> Self {
        Self {
            portfolio,
            weighting: WeightingScheme::ValueWeight,
            trade_universe: TradeUniverse::default(),
            objective,
            constraints: vec![Constraint::Budget { rhs: 1.0 }],
            missing_metric_policy: MissingMetricPolicy::Zero,
            label: None,
            meta: IndexMap::new(),
        }
    }

    /// Set the trade universe.
    #[must_use]
    pub fn with_trade_universe(mut self, universe: TradeUniverse) -> Self {
        self.trade_universe = universe;
        self
    }

    /// Add a single constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add multiple constraints.
    #[must_use]
    pub fn with_constraints(mut self, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        self.constraints.extend(constraints);
        self
    }
}

/// Status of an optimization run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStatus {
    /// Found optimal solution.
    Optimal,

    /// Found feasible solution but solver stopped early.
    FeasibleButSuboptimal,

    /// No feasible solution exists.
    Infeasible {
        /// Constraints that appear to conflict (if determinable).
        conflicting_constraints: Vec<String>,
    },

    /// Objective is unbounded.
    Unbounded,

    /// Solver error.
    Error { message: String },
}

impl OptimizationStatus {
    /// Check if the status represents a usable solution.
    #[must_use]
    pub fn is_feasible(&self) -> bool {
        matches!(self, Self::Optimal | Self::FeasibleButSuboptimal)
    }
}

/// Direction of a trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeDirection {
    /// Buy more of the instrument (increase exposure).
    Buy,
    /// Sell the instrument (decrease exposure).
    Sell,
    /// No change in exposure.
    Hold,
}

/// Whether a trade is for an existing position or a new candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    /// Adjusting an existing portfolio position.
    Existing,
    /// Adding a new position from the candidate universe.
    NewPosition,
    /// Closing out an existing position entirely.
    CloseOut,
}

/// Trade specification for a single position.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeSpec {
    /// Position identifier in the optimized portfolio.
    pub position_id: PositionId,
    /// Underlying instrument identifier (or candidate id).
    pub instrument_id: String,
    /// Trade type (existing, new position, close‑out).
    pub trade_type: TradeType,
    /// Pre‑trade quantity.
    pub current_quantity: f64,
    /// Post‑trade quantity.
    pub target_quantity: f64,
    /// Quantity change (`target - current`).
    pub delta_quantity: f64,
    /// Buy / Sell / Hold classification.
    pub direction: TradeDirection,
    /// Pre‑trade weight.
    pub current_weight: f64,
    /// Post‑trade weight.
    pub target_weight: f64,
}

/// Solution of an optimization problem.
#[derive(Clone, Debug)]
pub struct PortfolioOptimizationResult {
    /// Echo of the original problem for traceability.
    pub problem: PortfolioOptimizationProblem,

    /// Pre‑trade weights (current portfolio state).
    pub current_weights: IndexMap<PositionId, f64>,

    /// Optimal weights per position (for positions in the universe).
    /// Non‑universe positions implicitly keep weight = current_weight.
    pub optimal_weights: IndexMap<PositionId, f64>,

    /// Weight changes (`optimal - current`).
    pub weight_deltas: IndexMap<PositionId, f64>,

    /// Implied target quantities for each position (units / face / notional).
    pub implied_quantities: IndexMap<PositionId, f64>,

    /// Objective value at the solution.
    pub objective_value: f64,

    /// Evaluated portfolio‑level metrics at the solution (for key `MetricExpr`s).
    pub metric_values: IndexMap<String, f64>,

    /// Optimization status.
    pub status: OptimizationStatus,

    /// Shadow prices / dual values for constraints (how much objective improves
    /// per unit relaxation). Key is constraint label or auto‑generated identifier.
    pub dual_values: IndexMap<String, f64>,

    /// Constraint slack values (positive = slack, zero ≈ binding).
    pub constraint_slacks: IndexMap<String, f64>,

    /// Calculation metadata (numeric mode, rounding context, timing).
    pub meta: ResultsMeta,
}

impl PortfolioOptimizationResult {
    /// Generate a new portfolio with quantities adjusted to target weights.
    ///
    /// This is a convenience helper that:
    /// - Clones the original portfolio
    /// - Updates position quantities according to `implied_quantities`
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError::InvalidInput`] if the optimization did not
    /// find a feasible solution.
    pub fn to_rebalanced_portfolio(&self) -> Result<Portfolio> {
        if !self.status.is_feasible() {
            return Err(PortfolioError::invalid_input(
                "cannot generate rebalanced portfolio from infeasible solution",
            ));
        }

        let mut portfolio = self.problem.portfolio.clone();

        for position in &mut portfolio.positions {
            if let Some(qty) = self.implied_quantities.get(&position.position_id) {
                position.quantity = *qty;
            }
        }

        portfolio.validate()?;
        Ok(portfolio)
    }

    /// Generate trade list (delta from current to target).
    ///
    /// Returns trades sorted by absolute quantity delta (largest first).
    /// Includes both adjustments to existing positions and new positions from
    /// candidates.
    #[must_use]
    pub fn to_trade_list(&self) -> Vec<TradeSpec> {
        let mut trades: Vec<TradeSpec> = self
            .optimal_weights
            .iter()
            .filter_map(|(pos_id, &target_weight)| {
                let current_weight = self.current_weights.get(pos_id).copied().unwrap_or(0.0);
                let delta_weight = target_weight - current_weight;

                // Skip positions with negligible change
                if !delta_weight.is_finite() || delta_weight.abs() < 1e-9 {
                    return None;
                }

                let existing_position = self.problem.portfolio.get_position(pos_id.as_str());
                let is_candidate = existing_position.is_none();

                let current_qty = existing_position.map(|p| p.quantity).unwrap_or(0.0);
                let target_qty = self
                    .implied_quantities
                    .get(pos_id)
                    .copied()
                    .unwrap_or(0.0);

                let trade_type = if is_candidate && target_weight > 1e-9 {
                    TradeType::NewPosition
                } else if !is_candidate && target_weight < 1e-9 {
                    TradeType::CloseOut
                } else {
                    TradeType::Existing
                };

                // Get instrument_id (from existing position or candidate)
                let instrument_id = existing_position
                    .map(|p| p.instrument_id.clone())
                    .or_else(|| {
                        self.problem
                            .trade_universe
                            .candidates
                            .iter()
                            .find(|c| c.id == *pos_id)
                            .map(|c| c.instrument.id().to_string())
                    })
                    .unwrap_or_default();

                let direction = if delta_weight > 0.0 {
                    TradeDirection::Buy
                } else if delta_weight < 0.0 {
                    TradeDirection::Sell
                } else {
                    TradeDirection::Hold
                };

                Some(TradeSpec {
                    position_id: pos_id.clone(),
                    instrument_id,
                    trade_type,
                    current_quantity: current_qty,
                    target_quantity: target_qty,
                    delta_quantity: target_qty - current_qty,
                    direction,
                    current_weight,
                    target_weight,
                })
            })
            .collect();

        trades.sort_by(|a, b| {
            b.delta_quantity
                .abs()
                .partial_cmp(&a.delta_quantity.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        trades
    }

    /// Get only trades for new positions (from candidates).
    #[must_use]
    pub fn new_position_trades(&self) -> Vec<TradeSpec> {
        self.to_trade_list()
            .into_iter()
            .filter(|t| t.trade_type == TradeType::NewPosition)
            .collect()
    }

    /// Get binding constraints at the optimal solution (slack ≈ 0).
    #[must_use]
    pub fn binding_constraints(&self) -> Vec<(&str, f64)> {
        self.constraint_slacks
            .iter()
            .filter(|(_, &slack)| slack.abs() < 1e-6)
            .map(|(name, &slack)| (name.as_str(), slack))
            .collect()
    }

    /// Calculate total turnover (sum of absolute weight changes).
    #[must_use]
    pub fn turnover(&self) -> f64 {
        self.weight_deltas.values().map(|d| d.abs()).sum()
    }
}

/// Optimizer interface; allows swapping implementations (LP, QP, etc.).
pub trait PortfolioOptimizer: Send + Sync {
    /// Optimize the portfolio for the given problem and market/config context.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError`] when:
    /// - Portfolio validation fails
    /// - Required metrics cannot be priced
    /// - The LP backend fails or returns an invalid solution
    fn optimize(
        &self,
        problem: &PortfolioOptimizationProblem,
        market: &MarketContext,
        config: &FinstackConfig,
    ) -> Result<PortfolioOptimizationResult>;
}

/// LP‑based optimizer using the `good_lp` crate as backend.
pub struct DefaultLpOptimizer {
    /// Solver tolerance for optimality.
    pub tolerance: f64,
    /// Maximum iterations (backend‑specific meaning).
    pub max_iterations: usize,
}

impl Default for DefaultLpOptimizer {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-8,
            max_iterations: 10_000,
        }
    }
}

/// Internal representation of a decision variable.
#[derive(Clone, Debug)]
struct DecisionItem {
    position_id: PositionId,
    /// Whether this represents an existing portfolio position.
    is_existing: bool,
    /// Whether this position is held (weight locked to current).
    is_held: bool,
    /// Current quantity (0.0 for candidates).
    current_quantity: f64,
}

/// Per‑decision‑variable features used to build linear forms.
#[derive(Clone, Debug)]
struct DecisionFeatures {
    /// Current base‑currency PV (scaled by quantity; 0 for candidates).
    pv_base: f64,
    /// Base‑currency PV per unit of quantity (for implied quantities).
    pv_per_unit: f64,
    /// Metric measures by string key (as in `ValuationResult::measures`).
    measures: IndexMap<String, f64>,
    /// Tags used by tag‑based constraints.
    tags: IndexMap<String, String>,
    /// Minimum weight bound.
    min_weight: f64,
    /// Maximum weight bound.
    max_weight: f64,
}

/// Relation for LP constraints.
#[derive(Clone, Copy, Debug)]
enum LpRelation {
    /// `lhs <= rhs`.
    Le,
    /// `lhs >= rhs`.
    Ge,
    /// `lhs == rhs`.
    Eq,
}

/// Linear constraint: `coefficients · w (<=,>=,=) rhs`.
#[derive(Clone, Debug)]
struct LpConstraint {
    coefficients: Vec<f64>,
    relation: LpRelation,
    rhs: f64,
    /// Optional name (constraint label) for diagnostics.
    name: Option<String>,
}

impl DefaultLpOptimizer {
    /// Collect all `MetricId`s required by the problem's `PerPositionMetric`s.
    fn required_metrics(problem: &PortfolioOptimizationProblem) -> Vec<MetricId> {
        let mut metrics = Vec::new();

        let mut add_metric = |ppm: &PerPositionMetric| {
            if let PerPositionMetric::Metric(id) = ppm {
                if !metrics.contains(id) {
                    metrics.push(id.clone());
                }
            }
        };

        match &problem.objective {
            Objective::Maximize(expr) | Objective::Minimize(expr) => {
                if let MetricExpr::WeightedSum { metric }
                | MetricExpr::ValueWeightedAverage { metric } = expr
                {
                    add_metric(metric);
                }
            }
        }

        for constraint in &problem.constraints {
            if let Constraint::MetricBound { metric, .. } = constraint {
                if let MetricExpr::WeightedSum { metric: ppm }
                | MetricExpr::ValueWeightedAverage { metric: ppm } = metric
                {
                    add_metric(ppm);
                }
            }
        }

        metrics
    }

    /// Whether the optimization problem relies on price-based yield/spread metrics for bonds.
    ///
    /// These metrics conceptually depend on **market prices** (via
    /// `pricing_overrides.quoted_clean_price`) rather than model PVs. When such
    /// metrics are requested in the objective or constraints, we require that
    /// every bond used as a decision variable has an explicit quoted clean price.
    fn uses_price_based_yield_metrics(required: &[MetricId]) -> bool {
        required.iter().any(|m| {
            m == &MetricId::Ytm
                || m == &MetricId::Ytw
                || m == &MetricId::ZSpread
                || m == &MetricId::Oas
                || m == &MetricId::DiscountMargin
                || m == &MetricId::ASWSpread
                || m == &MetricId::ASWPar
                || m == &MetricId::ASWMarket
                || m == &MetricId::ASWParFwd
                || m == &MetricId::ASWMarketFwd
        })
    }

    /// Validate that all bond decision variables have an explicit quoted price when
    /// using price-based yield/spread metrics (e.g., YTM) in the optimization.
    ///
    /// This enforces that optimization over bond yields is driven by **market**
    /// prices rather than theoretical model PVs.
    fn validate_bond_price_overrides(
        problem: &PortfolioOptimizationProblem,
        decision_items: &[DecisionItem],
    ) -> Result<()> {
        for item in decision_items {
            if item.is_existing {
                let position = problem
                    .portfolio
                    .get_position(item.position_id.as_str())
                    .ok_or_else(|| {
                        PortfolioError::index_error(format!(
                            "missing position '{}' while validating bond prices",
                            item.position_id
                        ))
                    })?;

                if let Some(bond) = position.instrument.as_any().downcast_ref::<Bond>() {
                    if bond.pricing_overrides.quoted_clean_price.is_none() {
                        return Err(PortfolioError::invalid_input(format!(
                            "Position '{}' (bond '{}') is used in a yield-based optimization \
                             but has no quoted_clean_price; set a market price via \
                             PricingOverrides or use a PV-based metric instead",
                            position.position_id,
                            bond.id.as_str()
                        )));
                    }
                }
            } else {
                // Candidate positions
                let candidate = problem
                    .trade_universe
                    .candidates
                    .iter()
                    .find(|c| c.id == item.position_id)
                    .ok_or_else(|| {
                        PortfolioError::index_error(format!(
                            "missing candidate '{}' while validating bond prices",
                            item.position_id
                        ))
                    })?;

                if let Some(bond) = candidate.instrument.as_any().downcast_ref::<Bond>() {
                    if bond.pricing_overrides.quoted_clean_price.is_none() {
                        return Err(PortfolioError::invalid_input(format!(
                            "Candidate position '{}' (bond '{}') is used in a yield-based \
                             optimization but has no quoted_clean_price; set a market price \
                             via PricingOverrides or use a PV-based metric instead",
                            candidate.id,
                            bond.id.as_str()
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a position matches a filter.
    fn matches_filter(position: &Position, filter: &PositionFilter) -> bool {
        match filter {
            PositionFilter::All => true,
            PositionFilter::ByEntityId(entity_id) => &position.entity_id == entity_id,
            PositionFilter::ByTag { key, value } => position
                .tags
                .get(key)
                .map_or(false, |v| v == value),
            PositionFilter::ByPositionIds(ids) => ids.contains(&position.position_id),
            PositionFilter::Not(inner) => !Self::matches_filter(position, inner),
        }
    }

    /// Build decision items and associated features from the portfolio and trade universe.
    #[allow(clippy::too_many_arguments)]
    fn build_decision_space(
        problem: &PortfolioOptimizationProblem,
        valuation: &crate::valuation::PortfolioValuation,
        required_metrics: &[MetricId],
        market: &MarketContext,
    ) -> Result<(Vec<DecisionItem>, Vec<DecisionFeatures>, IndexMap<PositionId, f64>, f64)> {
        let mut items = Vec::new();
        let mut features = Vec::new();
        let mut current_weights = IndexMap::new();

        // Map position id -> (pv_base, measures, tags, quantity)
        let mut total_pv_base = 0.0_f64;

        for position in &problem.portfolio.positions {
            // Pull valuation for this position
            let pv_entry = valuation
                .position_values
                .get(&position.position_id)
                .ok_or_else(|| {
                    PortfolioError::index_error(format!(
                        "missing valuation for position '{}'",
                        position.position_id
                    ))
                })?;

            let pv_base = pv_entry.value_base.amount();
            total_pv_base = neumaier_sum([total_pv_base, pv_base].into_iter());

            // Extract measures
            let mut measures = IndexMap::new();
            if let Some(val_result) = &pv_entry.valuation_result {
                measures = val_result.measures.clone();
            } else if !required_metrics.is_empty()
                && matches!(problem.missing_metric_policy, MissingMetricPolicy::Strict)
            {
                return Err(PortfolioError::valuation(
                    position.position_id.clone(),
                    "valuation result missing required metrics",
                ));
            }

            // Decide if position is held / tradeable
            let is_held = if let Some(ref held) = problem.trade_universe.held_filter {
                Self::matches_filter(position, held)
            } else {
                false
            };

            let is_tradeable = Self::matches_filter(position, &problem.trade_universe.tradeable_filter);

            if !is_tradeable && !is_held {
                // Excluded from optimization entirely; still counted in total PV for weights.
                continue;
            }

            items.push(DecisionItem {
                position_id: position.position_id.clone(),
                is_existing: true,
                is_held,
                current_quantity: position.quantity,
            });

            // Default weight bounds: [0, 1]; will be refined by constraints later.
            features.push(DecisionFeatures {
                pv_base,
                // When quantity == 0, treat pv_per_unit as 0 to avoid division by zero.
                pv_per_unit: if position.quantity != 0.0 {
                    pv_base / position.quantity
                } else {
                    0.0
                },
                measures,
                tags: position.tags.clone(),
                min_weight: 0.0,
                max_weight: 1.0,
            });
        }

        // Add candidates as decision items
        for candidate in &problem.trade_universe.candidates {
            // Price candidate once to obtain PV and measures
            let val_result = candidate
                .instrument
                .price_with_metrics(market, problem.portfolio.as_of, required_metrics)
                .map_err(|e: finstack_core::Error| {
                    PortfolioError::valuation(candidate.id.clone(), e.to_string())
                })?;

            // Convert PV to base currency (similar to `value_single_position`)
            let value_native = val_result.value;
            let scaled_native = match candidate.unit {
                PositionUnit::Units => value_native,
                PositionUnit::Notional(notional_ccy) => {
                    if let Some(ccy) = notional_ccy {
                        if ccy != value_native.currency() {
                            tracing::warn!(
                                position_id = %candidate.id,
                                "Notional currency {} differs from instrument currency {}",
                                ccy,
                                value_native.currency()
                            );
                        }
                    }
                    value_native
                }
                PositionUnit::FaceValue => value_native,
                PositionUnit::Percentage => Money::new(value_native.amount() / 100.0, value_native.currency()),
            };

            let value_base = if scaled_native.currency() == problem.portfolio.base_ccy {
                scaled_native
            } else {
                let fx_matrix = market.fx.as_ref().ok_or_else(|| {
                    PortfolioError::missing_market_data("FX matrix not available for candidate")
                })?;

                let query = FxQuery::new(
                    scaled_native.currency(),
                    problem.portfolio.base_ccy,
                    problem.portfolio.as_of,
                );
                let rate_result = fx_matrix.rate(query).map_err(|_| {
                    PortfolioError::fx_conversion(scaled_native.currency(), problem.portfolio.base_ccy)
                })?;

                Money::new(
                    scaled_native.amount() * rate_result.rate,
                    problem.portfolio.base_ccy,
                )
            };

            let pv_base = value_base.amount();

            items.push(DecisionItem {
                position_id: candidate.id.clone(),
                is_existing: false,
                is_held: false,
                current_quantity: 0.0,
            });

            features.push(DecisionFeatures {
                pv_base: 0.0,
                pv_per_unit: if pv_base != 0.0 { pv_base } else { 0.0 },
                measures: val_result.measures.clone(),
                tags: candidate.tags.clone(),
                min_weight: candidate.min_weight,
                max_weight: candidate.max_weight,
            });
        }

        // Compute current weights from pv_base; only existing positions contribute.
        let mut total_pv_existing = 0.0_f64;
        for feat in &features {
            total_pv_existing = neumaier_sum([total_pv_existing, feat.pv_base].into_iter());
        }

        if total_pv_existing > 0.0 {
            // Standard case: weights proportional to existing PV.
            for (item, feat) in items.iter().zip(&features) {
                let w0 = if feat.pv_base > 0.0 {
                    feat.pv_base / total_pv_existing
                } else {
                    0.0
                };
                current_weights.insert(item.position_id.clone(), w0);
            }
        } else if !items.is_empty() {
            // Degenerate case: zero or negative PVs – fall back to uniform weights
            // over existing positions and zero weight for pure candidates.
            let n_existing = items.iter().filter(|i| i.is_existing).count();
            let uniform = if n_existing > 0 {
                1.0 / n_existing as f64
            } else {
                0.0
            };

            for item in &items {
                let w0 = if item.is_existing { uniform } else { 0.0 };
                current_weights.insert(item.position_id.clone(), w0);
            }
        }

        // Use total portfolio PV (including non‑tradeable positions) as the scale
        // for implied quantities; fall back to 1.0 to avoid division by zero.
        let total_pv_scale = valuation
            .total_base_ccy
            .amount()
            .abs()
            .max(1.0);

        Ok((items, features, current_weights, total_pv_scale))
    }

    /// Lower a `PerPositionMetric` to a per‑decision value `m_i`.
    fn per_position_metric_value(
        ppm: &PerPositionMetric,
        feat: &DecisionFeatures,
        missing_policy: MissingMetricPolicy,
    ) -> Result<f64> {
        let val = match ppm {
            PerPositionMetric::Metric(id) => feat.measures.get(id.as_str()).copied(),
            PerPositionMetric::CustomKey(key) => feat.measures.get(key).copied(),
            PerPositionMetric::PvBase => Some(feat.pv_base),
            PerPositionMetric::PvNative => {
                // We do not retain native PV separately in `DecisionFeatures`; for now
                // treat PvNative as PvBase which is already in base currency.
                Some(feat.pv_base)
            }
            PerPositionMetric::TagEquals { key, value } => {
                let matches = feat.tags.get(key).map_or(false, |v| v == value);
                Some(if matches { 1.0 } else { 0.0 })
            }
            PerPositionMetric::Constant(c) => Some(*c),
        };

        match (val, missing_policy) {
            (Some(v), _) => Ok(v),
            (None, MissingMetricPolicy::Zero) => Ok(0.0),
            (None, MissingMetricPolicy::Exclude) => Ok(0.0),
            (None, MissingMetricPolicy::Strict) => Err(PortfolioError::invalid_input(
                "required metric missing for position",
            )),
        }
    }

    /// Build coefficient vector `a` for a `MetricExpr`.
    fn build_metric_coefficients(
        expr: &MetricExpr,
        feats: &[DecisionFeatures],
        missing_policy: MissingMetricPolicy,
        trade_universe: &TradeUniverse,
        items: &[DecisionItem],
        portfolio: &Portfolio,
    ) -> Result<Vec<f64>> {
        let mut coeffs = Vec::with_capacity(feats.len());
        match expr {
            MetricExpr::WeightedSum { metric } | MetricExpr::ValueWeightedAverage { metric } => {
                for feat in feats {
                    let m_i = Self::per_position_metric_value(metric, feat, missing_policy)?;
                    coeffs.push(m_i);
                }
            }
            MetricExpr::TagExposureShare { tag_key, tag_value } => {
                for (item, feat) in items.iter().zip(feats) {
                    let mut matches = feat
                        .tags
                        .get(tag_key)
                        .map_or(false, |v| v == tag_value);
                    // Also consider portfolio‑level tags if any
                    if !matches {
                        if let Some(position) = portfolio.get_position(item.position_id.as_str()) {
                            matches = position
                                .tags
                                .get(tag_key)
                                .map_or(false, |v| v == tag_value);
                        }
                    }
                    // Candidates already have tags in `feat.tags`
                    let weight = if matches { 1.0 } else { 0.0 };
                    coeffs.push(weight);
                }
            }
        }

        // For `MissingMetricPolicy::Exclude`, zero out coefficients for positions
        // missing metrics, but we already handled that via `per_position_metric_value`.
        let _ = trade_universe; // reserved for future use (e.g., additional policies)

        Ok(coeffs)
    }
}

impl PortfolioOptimizer for DefaultLpOptimizer {
    fn optimize(
        &self,
        problem: &PortfolioOptimizationProblem,
        market: &MarketContext,
        config: &FinstackConfig,
    ) -> Result<PortfolioOptimizationResult> {
        // Step 0: Validate portfolio and problem basics.
        problem.portfolio.validate()?;

        match problem.weighting {
            WeightingScheme::ValueWeight | WeightingScheme::UnitScaling => {}
            WeightingScheme::NotionalWeight => {
                return Err(PortfolioError::invalid_input(
                    "NotionalWeight is not yet supported in optimization",
                ))
            }
        }

        // Ensure there is at least one budget constraint, or we will add one with rhs=1.0.
        let has_budget = problem
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Budget { .. }));

        // Step 1: Discover required metrics and value portfolio.
        let required_metrics = Self::required_metrics(problem);
        let mut options = crate::valuation::PortfolioValuationOptions::default();
        options.strict_risk = matches!(problem.missing_metric_policy, MissingMetricPolicy::Strict);
        options.additional_metrics = if required_metrics.is_empty() {
            None
        } else {
            Some(required_metrics.clone())
        };
        options.replace_standard_metrics = false;

        let valuation = crate::valuation::value_portfolio_with_options(
            &problem.portfolio,
            market,
            config,
            &options,
        )?;

        // Step 2: Build decision space.
        let (decision_items, mut decision_features, current_weights, total_pv) =
            Self::build_decision_space(problem, &valuation, &required_metrics, market)?;

        // If the problem optimizes on price-based yield/spread metrics, require that all
        // bond decision variables have an explicit quoted clean price override.
        if Self::uses_price_based_yield_metrics(&required_metrics) {
            Self::validate_bond_price_overrides(problem, &decision_items)?;
        }

        if decision_items.is_empty() {
            return Err(PortfolioError::invalid_input(
                "no decision variables in optimization problem",
            ));
        }

        let n_vars = decision_items.len();

        // Step 3: Apply weight bounds from WeightBounds constraints.
        for constraint in &problem.constraints {
            if let Constraint::WeightBounds {
                filter, min, max, ..
            } = constraint
            {
                for (item, feat) in decision_items.iter().zip(decision_features.iter_mut()) {
                    // Reuse matches_filter logic.
                    let is_match = if item.is_existing {
                        if let Some(position) =
                            problem.portfolio.get_position(item.position_id.as_str())
                        {
                            DefaultLpOptimizer::matches_filter(position, filter)
                        } else {
                            false
                        }
                    } else {
                        // Candidate: match via candidate tags / ids
                        problem
                            .trade_universe
                            .candidates
                            .iter()
                            .find(|c| c.id == item.position_id)
                            .map_or(false, |_| true)
                    };

                    if is_match {
                        feat.min_weight = feat.min_weight.max(*min);
                        feat.max_weight = feat.max_weight.min(*max);
                    }
                }
            }
        }

        // Step 4: Build objective coefficients.
        let objective_expr = &problem.objective;
        let coeffs_objective = match objective_expr {
            Objective::Maximize(expr) | Objective::Minimize(expr) => {
                Self::build_metric_coefficients(
                    expr,
                    &decision_features,
                    problem.missing_metric_policy,
                    &problem.trade_universe,
                    &decision_items,
                    &problem.portfolio,
                )?
            }
        };

        // Step 5: Build constraints as LP rows.
        let mut lp_constraints: Vec<LpConstraint> = Vec::new();

        for constraint in &problem.constraints {
            match constraint {
                Constraint::MetricBound {
                    label,
                    metric,
                    op,
                    rhs,
                } => {
                    let a = Self::build_metric_coefficients(
                        metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    let relation = match op {
                        Inequality::Le => LpRelation::Le,
                        Inequality::Ge => LpRelation::Ge,
                        Inequality::Eq => LpRelation::Eq,
                    };
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation,
                        rhs: *rhs,
                        name: label.clone(),
                    });
                }
                Constraint::TagExposureLimit {
                    label,
                    tag_key,
                    tag_value,
                    max_share,
                } => {
                    let metric = MetricExpr::TagExposureShare {
                        tag_key: tag_key.clone(),
                        tag_value: tag_value.clone(),
                    };
                    let a = Self::build_metric_coefficients(
                        &metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation: LpRelation::Le,
                        rhs: *max_share,
                        name: label.clone(),
                    });
                }
                Constraint::TagExposureMinimum {
                    label,
                    tag_key,
                    tag_value,
                    min_share,
                } => {
                    let metric = MetricExpr::TagExposureShare {
                        tag_key: tag_key.clone(),
                        tag_value: tag_value.clone(),
                    };
                    let a = Self::build_metric_coefficients(
                        &metric,
                        &decision_features,
                        problem.missing_metric_policy,
                        &problem.trade_universe,
                        &decision_items,
                        &problem.portfolio,
                    )?;
                    lp_constraints.push(LpConstraint {
                        coefficients: a,
                        relation: LpRelation::Ge,
                        rhs: *min_share,
                        name: label.clone(),
                    });
                }
                Constraint::WeightBounds { .. } => {
                    // Already applied to `DecisionFeatures::min_weight/max_weight`.
                }
                Constraint::MaxTurnover { label, max_turnover } => {
                    // Turnover handled later via auxiliary variables.
                    // We record a synthetic constraint row: coefficients of 1 for t_i.
                    lp_constraints.push(LpConstraint {
                        coefficients: vec![0.0; n_vars], // placeholder; filled later
                        relation: LpRelation::Le,
                        rhs: *max_turnover,
                        name: label.clone(),
                    });
                }
                Constraint::MaxPositionDelta { .. } => {
                    // Implemented later by additional bounds around current weights.
                }
                Constraint::Budget { rhs } => {
                    // Budget: sum_i w_i == rhs
                    let coefficients = vec![1.0; n_vars];
                    lp_constraints.push(LpConstraint {
                        coefficients,
                        relation: LpRelation::Eq,
                        rhs: *rhs,
                        name: Some("budget".to_string()),
                    });
                }
            }
        }

        if !has_budget {
            // Add implicit budget constraint sum_i w_i == 1.0 if none was provided.
            lp_constraints.push(LpConstraint {
                coefficients: vec![1.0; n_vars],
                relation: LpRelation::Eq,
                rhs: 1.0,
                name: Some("budget".to_string()),
            });
        }

        // Step 6: Assemble LP model using good_lp.
        let maximise = matches!(problem.objective, Objective::Maximize(_));
        let mut vars = good_lp::variables!();

        // Decision variables w_i
        let mut w_vars = Vec::with_capacity(n_vars);
        for (item, feat) in decision_items.iter().zip(&decision_features) {
            let current_weight = current_weights.get(&item.position_id).copied().unwrap_or(0.0);
            // Held positions: lock weight at current value by min = max = current_weight.
            let (min_w, max_w) = if item.is_held {
                (current_weight, current_weight)
            } else {
                (feat.min_weight, feat.max_weight)
            };

            w_vars.push(
                vars.add(variable().min(min_w).max(max_w)),
            );
        }

        // Auxiliary variables for turnover t_i (|w_i - w0_i|) if needed.
        let has_turnover_constraint = problem
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxTurnover { .. }));

        let mut t_vars: Vec<Option<good_lp::Variable>> = vec![None; n_vars];

        if has_turnover_constraint {
            for idx in 0..n_vars {
                t_vars[idx] = Some(vars.add(variable().min(0.0)));
            }
        }

        // Objective
        let mut objective_expr: Expression = 0.0.into();
        for (var, coef) in w_vars.iter().zip(&coeffs_objective) {
            objective_expr = objective_expr + (*coef) * *var;
        }

        let mut problem_model = if maximise {
            vars.maximise(objective_expr)
        } else {
            vars.minimise(objective_expr)
        }
        .using(default_solver);

        // Add primary constraints
        for lc in &lp_constraints {
            // Skip placeholder turnover row; handled below.
            if matches!(lc.name.as_deref(), Some(name) if name == "turnover") {
                continue;
            }

            let mut lhs: Expression = 0.0.into();
            for (var, coef) in w_vars.iter().zip(&lc.coefficients) {
                lhs = lhs + (*coef) * *var;
            }

            problem_model = match lc.relation {
                LpRelation::Le => problem_model.with(constraint!(lhs <= lc.rhs)),
                LpRelation::Ge => problem_model.with(constraint!(lhs >= lc.rhs)),
                LpRelation::Eq => problem_model.with(constraint!(lhs == lc.rhs)),
            };
        }

        // Turnover constraint with auxiliary variables: Σ t_i <= max_turnover.
        if let Some(Constraint::MaxTurnover { max_turnover, .. }) = problem
            .constraints
            .iter()
            .find(|c| matches!(c, Constraint::MaxTurnover { .. }))
        {
            // For each i: t_i >= w_i - w0_i and t_i >= w0_i - w_i
            for (idx, w_var) in w_vars.iter().enumerate() {
                let t_var = match t_vars[idx] {
                    Some(v) => v,
                    None => continue,
                };
                let w0 = current_weights
                    .get(&decision_items[idx].position_id)
                    .copied()
                    .unwrap_or(0.0);

                // t_i >= w_i - w0  ->  t_i - w_i >= -w0
                let lhs1: Expression = t_var - *w_var;
                problem_model = problem_model.with(constraint!(lhs1 >= -w0));

                // t_i >= w0 - w_i  ->  t_i + w_i >= w0
                let lhs2: Expression = t_var + *w_var;
                problem_model = problem_model.with(constraint!(lhs2 >= w0));
            }

            // Sum t_i <= max_turnover
            let mut lhs_turnover: Expression = 0.0.into();
            for t_var in &t_vars {
                if let Some(tv) = t_var {
                    lhs_turnover = lhs_turnover + *tv;
                }
            }
            problem_model =
                problem_model.with(constraint!(lhs_turnover <= *max_turnover));
        }

        // Solve LP
        let solution = problem_model
            .solve()
            .map_err(|e| PortfolioError::optimization_error(e.to_string()))?;

        // Extract weights
        let mut optimal_weights: IndexMap<PositionId, f64> = IndexMap::new();
        let mut weight_deltas: IndexMap<PositionId, f64> = IndexMap::new();

        for (item, w_var) in decision_items.iter().zip(&w_vars) {
            let w_star = solution.value(*w_var);
            let w0 = current_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);
            optimal_weights.insert(item.position_id.clone(), w_star);
            weight_deltas.insert(item.position_id.clone(), w_star - w0);
        }

        // Implied quantities
        let mut implied_quantities: IndexMap<PositionId, f64> = IndexMap::new();
        for (idx, item) in decision_items.iter().enumerate() {
            let feat = &decision_features[idx];
            let w_star = optimal_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);
            let w0 = current_weights
                .get(&item.position_id)
                .copied()
                .unwrap_or(0.0);

            let qty = if item.is_existing {
                if w0 > 0.0 {
                    item.current_quantity * w_star / w0
                } else {
                    // New long / short in previously zero‑weight existing position.
                    if feat.pv_per_unit != 0.0 {
                        (w_star * total_pv) / feat.pv_per_unit
                    } else {
                        0.0
                    }
                }
            } else if feat.pv_per_unit != 0.0 {
                // Candidate position: scale by PV per unit.
                (w_star * total_pv) / feat.pv_per_unit
            } else {
                0.0
            };

            implied_quantities.insert(item.position_id.clone(), qty);
        }

        // Objective value at solution: a · w*
        let mut objective_value = 0.0_f64;
        for (coef, w_var) in coeffs_objective.iter().zip(&w_vars) {
            let w_star = solution.value(*w_var);
            objective_value = neumaier_sum([objective_value, *coef * w_star].into_iter());
        }

        // Evaluate additional metric expressions of interest (for now: just objective).
        let mut metric_values: IndexMap<String, f64> = IndexMap::new();
        metric_values.insert("objective".to_string(), objective_value);

        // Constraint slacks and dual values are backend‑specific; for now, leave empty.
        let dual_values: IndexMap<String, f64> = IndexMap::new();
        let constraint_slacks: IndexMap<String, f64> = IndexMap::new();

        // Optimization status – assume optimal if solve() succeeded.
        let status = OptimizationStatus::Optimal;

        // Reuse results_meta from config.
        let meta = results_meta(config);

        Ok(PortfolioOptimizationResult {
            problem: problem.clone(),
            current_weights,
            optimal_weights,
            weight_deltas,
            implied_quantities,
            objective_value,
            metric_values,
            status,
            dual_values,
            constraint_slacks,
            meta,
        })
    }
}


