//! Python bindings for portfolio optimization.
//!
//! This module provides comprehensive bindings for the Rust optimization framework,
//! including constraint types, objectives, trade universe, and optimization problems.

use crate::core::config::{extract_config_or_default, PyResultsMeta};
use crate::core::market_data::context::PyMarketContext;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::{extract_portfolio, PyPortfolio};
use crate::portfolio::types::PyPositionUnit;
use crate::valuations::instruments::extract_instrument;
use crate::valuations::metrics::ids::PyMetricId;
use finstack_portfolio::optimization::{
    optimize_max_yield_with_ccc_limit, CandidatePosition, Constraint, DefaultLpOptimizer,
    Inequality, MaxYieldWithCccLimitResult, MetricExpr, MissingMetricPolicy, Objective,
    OptimizationStatus, PerPositionMetric, PortfolioOptimizationProblem,
    PortfolioOptimizationResult, PortfolioOptimizer, PositionFilter, TradeDirection, TradeSpec,
    TradeType, TradeUniverse, WeightingScheme,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash_discriminant<T>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    std::mem::discriminant(value).hash(&mut hasher);
    hasher.finish()
}

/// Optimize a bond portfolio to maximize value‑weighted YTM with a CCC exposure limit.
///
/// This helper mirrors the Rust example and the integration test:
///
/// - Objective: maximize value‑weighted average yield (`MetricId::Ytm`)
/// - Constraint: rating = "CCC" exposure (by weight) <= `ccc_limit`
/// - Budget: implicit `sum_i w_i == 1` via the default problem constructor
///
/// The portfolio should:
/// - Be USD‑denominated (base_ccy = USD)
/// - Contain bond‑like instruments that expose `MetricId::Ytm`
/// - Tag high‑yield positions with `rating="CCC"` for the constraint
#[pyfunction]
#[pyo3(
    name = "optimize_max_yield_with_ccc_limit",
    signature = (portfolio, market_context, ccc_limit=0.20, strict_risk=false, config=None)
)]
fn py_optimize_max_yield_with_ccc_limit(
    portfolio: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    ccc_limit: f64,
    strict_risk: bool,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyMaxYieldWithCccLimitResult> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;
    let cfg = extract_config_or_default(config)?;

    let result = optimize_max_yield_with_ccc_limit(
        &portfolio_inner,
        &market_ctx.inner,
        &cfg,
        ccc_limit,
        strict_risk,
    )
    .map_err(portfolio_to_py)?;

    Ok(PyMaxYieldWithCccLimitResult { inner: result })
}

fn map_weights(
    py: Python<'_>,
    weights: &indexmap::IndexMap<finstack_portfolio::types::PositionId, f64>,
) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (pos_id, weight) in weights {
        dict.set_item(pos_id.as_str(), *weight)?;
    }
    Ok(dict.into())
}

// ===========================
// Enums and Basic Types
// ===========================

/// How optimization weights are defined.
///
/// Examples
/// --------
/// >>> from finstack import WeightingScheme
/// >>> WeightingScheme.VALUE_WEIGHT  # w_i is share of portfolio PV
/// >>> WeightingScheme.NOTIONAL_WEIGHT  # w_i is share of notional exposure
/// >>> WeightingScheme.UNIT_SCALING  # w_i scales current position size
#[pyclass(
    name = "WeightingScheme",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyWeightingScheme {
    pub inner: WeightingScheme,
}

#[pymethods]
impl PyWeightingScheme {
    #[classattr]
    const VALUE_WEIGHT: Self = Self {
        inner: WeightingScheme::ValueWeight,
    };

    #[classattr]
    const NOTIONAL_WEIGHT: Self = Self {
        inner: WeightingScheme::NotionalWeight,
    };

    #[classattr]
    const UNIT_SCALING: Self = Self {
        inner: WeightingScheme::UnitScaling,
    };

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }
}

/// Policy for handling positions missing required metrics.
///
/// Examples
/// --------
/// >>> from finstack import MissingMetricPolicy
/// >>> MissingMetricPolicy.ZERO  # Treat missing as 0.0 (default)
/// >>> MissingMetricPolicy.EXCLUDE  # Exclude position from constraint
/// >>> MissingMetricPolicy.STRICT  # Fail with error if missing
#[pyclass(
    name = "MissingMetricPolicy",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMissingMetricPolicy {
    pub inner: MissingMetricPolicy,
}

#[pymethods]
impl PyMissingMetricPolicy {
    #[classattr]
    const ZERO: Self = Self {
        inner: MissingMetricPolicy::Zero,
    };

    #[classattr]
    const EXCLUDE: Self = Self {
        inner: MissingMetricPolicy::Exclude,
    };

    #[classattr]
    const STRICT: Self = Self {
        inner: MissingMetricPolicy::Strict,
    };

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }
}

/// Inequality/equality operator for constraints.
///
/// Examples
/// --------
/// >>> from finstack import Inequality
/// >>> Inequality.LE  # Less-than or equal: <=
/// >>> Inequality.GE  # Greater-than or equal: >=
/// >>> Inequality.EQ  # Equality: ==
#[pyclass(
    name = "Inequality",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyInequality {
    pub inner: Inequality,
}

#[pymethods]
impl PyInequality {
    #[classattr]
    const LE: Self = Self {
        inner: Inequality::Le,
    };

    #[classattr]
    const GE: Self = Self {
        inner: Inequality::Ge,
    };

    #[classattr]
    const EQ: Self = Self {
        inner: Inequality::Eq,
    };

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }
}

/// Optimization status enum.
///
/// Examples
/// --------
/// >>> result.status == OptimizationStatus.OPTIMAL
/// True
#[pyclass(
    name = "OptimizationStatus",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyOptimizationStatus {
    pub inner: OptimizationStatus,
}

#[pymethods]
impl PyOptimizationStatus {
    #[classattr]
    const OPTIMAL: Self = Self {
        inner: OptimizationStatus::Optimal,
    };

    #[classattr]
    const FEASIBLE_BUT_SUBOPTIMAL: Self = Self {
        inner: OptimizationStatus::FeasibleButSuboptimal,
    };

    #[classattr]
    const UNBOUNDED: Self = Self {
        inner: OptimizationStatus::Unbounded,
    };

    fn is_feasible(&self) -> bool {
        self.inner.is_feasible()
    }

    /// Return a string identifying the variant (e.g. "Optimal", "Infeasible").
    fn status_name(&self) -> &str {
        match &self.inner {
            OptimizationStatus::Optimal => "Optimal",
            OptimizationStatus::FeasibleButSuboptimal => "FeasibleButSuboptimal",
            OptimizationStatus::Infeasible { .. } => "Infeasible",
            OptimizationStatus::Unbounded => "Unbounded",
            OptimizationStatus::Error { .. } => "Error",
        }
    }

    /// Return conflicting constraint labels (only for Infeasible status).
    fn conflicting_constraints(&self) -> Vec<String> {
        match &self.inner {
            OptimizationStatus::Infeasible {
                conflicting_constraints,
            } => conflicting_constraints.clone(),
            _ => vec![],
        }
    }

    /// Return error message (only for Error status).
    fn error_message(&self) -> Option<String> {
        match &self.inner {
            OptimizationStatus::Error { message } => Some(message.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }
}

/// Direction of a trade.
#[pyclass(
    name = "TradeDirection",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyTradeDirection {
    pub inner: TradeDirection,
}

#[pymethods]
impl PyTradeDirection {
    #[classattr]
    const BUY: Self = Self {
        inner: TradeDirection::Buy,
    };

    #[classattr]
    const SELL: Self = Self {
        inner: TradeDirection::Sell,
    };

    #[classattr]
    const HOLD: Self = Self {
        inner: TradeDirection::Hold,
    };

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }
}

/// Whether a trade is for an existing position or a new candidate.
#[pyclass(
    name = "TradeType",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyTradeType {
    pub inner: TradeType,
}

#[pymethods]
impl PyTradeType {
    #[classattr]
    const EXISTING: Self = Self {
        inner: TradeType::Existing,
    };

    #[classattr]
    const NEW_POSITION: Self = Self {
        inner: TradeType::NewPosition,
    };

    #[classattr]
    const CLOSE_OUT: Self = Self {
        inner: TradeType::CloseOut,
    };

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_discriminant(&self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.inner) == std::mem::discriminant(&other.inner)
    }
}

// ===========================
// Per-Position Metrics
// ===========================

/// Where a per‑position scalar metric comes from.
///
/// Examples
/// --------
/// >>> from finstack import PerPositionMetric, MetricId
/// >>> PerPositionMetric.metric(MetricId.DURATION_MOD)  # Modified duration
/// >>> PerPositionMetric.metric(MetricId.YTM)  # Yield to maturity
/// >>> PerPositionMetric.pv_base()  # Base currency PV
/// >>> PerPositionMetric.constant(1.0)  # Constant for all positions
#[pyclass(
    name = "PerPositionMetric",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPerPositionMetric {
    pub inner: PerPositionMetric,
}

#[pymethods]
impl PyPerPositionMetric {
    /// Create from a standard MetricId.
    #[staticmethod]
    fn metric(metric_id: &PyMetricId) -> Self {
        Self {
            inner: PerPositionMetric::Metric(metric_id.inner.clone()),
        }
    }

    /// Create from a custom string key.
    #[staticmethod]
    fn custom_key(key: String) -> Self {
        Self {
            inner: PerPositionMetric::CustomKey(key),
        }
    }

    /// Use base currency PV.
    #[staticmethod]
    fn pv_base() -> Self {
        Self {
            inner: PerPositionMetric::PvBase,
        }
    }

    /// Use native currency PV.
    #[staticmethod]
    fn pv_native() -> Self {
        Self {
            inner: PerPositionMetric::PvNative,
        }
    }

    /// Tag-based 0/1 indicator.
    #[staticmethod]
    fn tag_equals(key: String, value: String) -> Self {
        Self {
            inner: PerPositionMetric::TagEquals { key, value },
        }
    }

    /// Constant scalar for all positions.
    #[staticmethod]
    fn constant(value: f64) -> Self {
        Self {
            inner: PerPositionMetric::Constant(value),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Metric Expressions
// ===========================

/// Portfolio‑level scalar metric expressed in terms of position metrics + weights.
///
/// Examples
/// --------
/// >>> from finstack import MetricExpr, PerPositionMetric, MetricId
/// >>> # Weighted sum of modified duration
/// >>> MetricExpr.weighted_sum(PerPositionMetric.metric(MetricId.DURATION_MOD))
/// >>> # Value-weighted average yield
/// >>> MetricExpr.value_weighted_average(PerPositionMetric.metric(MetricId.YTM))
/// >>> # Tag exposure share (e.g., HY bonds)
/// >>> MetricExpr.tag_exposure_share("rating", "HY")
#[pyclass(
    name = "MetricExpr",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMetricExpr {
    pub inner: MetricExpr,
}

#[pymethods]
impl PyMetricExpr {
    /// Weighted sum: sum_i w_i * m_i.
    #[staticmethod]
    fn weighted_sum(metric: PyPerPositionMetric) -> Self {
        Self {
            inner: MetricExpr::WeightedSum {
                metric: metric.inner,
            },
        }
    }

    /// Value-weighted average: sum_i w_i * m_i with implicit sum_i w_i == 1.
    #[staticmethod]
    fn value_weighted_average(metric: PyPerPositionMetric) -> Self {
        Self {
            inner: MetricExpr::ValueWeightedAverage {
                metric: metric.inner,
            },
        }
    }

    /// Tag exposure share: sum_i w_i * I[tag == value].
    #[staticmethod]
    fn tag_exposure_share(tag_key: String, tag_value: String) -> Self {
        Self {
            inner: MetricExpr::TagExposureShare { tag_key, tag_value },
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Objectives
// ===========================

/// Optimization direction and target.
///
/// Examples
/// --------
/// >>> from finstack import Objective, MetricExpr, PerPositionMetric, MetricId
/// >>> # Maximize value-weighted yield
/// >>> ytm_metric = PerPositionMetric.metric(MetricId.YTM)
/// >>> obj = Objective.maximize(MetricExpr.value_weighted_average(ytm_metric))
/// >>> # Minimize portfolio duration
/// >>> dur_metric = PerPositionMetric.metric(MetricId.DURATION_MOD)
/// >>> obj = Objective.minimize(MetricExpr.weighted_sum(dur_metric))
#[pyclass(
    name = "Objective",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyObjective {
    pub inner: Objective,
}

#[pymethods]
impl PyObjective {
    /// Maximize a portfolio-level metric.
    #[staticmethod]
    fn maximize(metric_expr: PyMetricExpr) -> Self {
        Self {
            inner: Objective::Maximize(metric_expr.inner),
        }
    }

    /// Minimize a portfolio-level metric.
    #[staticmethod]
    fn minimize(metric_expr: PyMetricExpr) -> Self {
        Self {
            inner: Objective::Minimize(metric_expr.inner),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Position Filters
// ===========================

/// Filters for selecting which positions are included in a constraint.
///
/// Examples
/// --------
/// >>> from finstack import PositionFilter
/// >>> PositionFilter.all()  # All positions
/// >>> PositionFilter.by_entity_id("fund_1")  # Single entity
/// >>> PositionFilter.by_tag("rating", "HY")  # Tagged positions
/// >>> PositionFilter.by_position_ids(["pos_1", "pos_2"])  # Specific positions
#[pyclass(
    name = "PositionFilter",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPositionFilter {
    pub inner: PositionFilter,
}

#[pymethods]
impl PyPositionFilter {
    /// All positions in the portfolio.
    #[staticmethod]
    fn all() -> Self {
        Self {
            inner: PositionFilter::All,
        }
    }

    /// Filter by entity ID.
    #[staticmethod]
    fn by_entity_id(entity_id: String) -> Self {
        Self {
            inner: PositionFilter::ByEntityId(entity_id.into()),
        }
    }

    /// Filter by tag key/value.
    #[staticmethod]
    fn by_tag(key: String, value: String) -> Self {
        Self {
            inner: PositionFilter::ByTag { key, value },
        }
    }

    /// Filter by multiple position IDs.
    #[staticmethod]
    fn by_position_ids(position_ids: Vec<String>) -> Self {
        Self {
            inner: PositionFilter::ByPositionIds(
                position_ids.into_iter().map(|id| id.into()).collect(),
            ),
        }
    }

    /// Exclude positions matching the inner filter.
    #[staticmethod]
    fn not_(filter: PyPositionFilter) -> Self {
        Self {
            inner: PositionFilter::Not(Box::new(filter.inner)),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Constraints
// ===========================

/// Declarative constraint specification.
///
/// Examples
/// --------
/// >>> from finstack import Constraint, MetricExpr, PerPositionMetric, MetricId, Inequality
/// >>> # Duration <= 4.0
/// >>> dur = PerPositionMetric.metric(MetricId.DURATION_MOD)
/// >>> c1 = Constraint.metric_bound("duration_limit", MetricExpr.weighted_sum(dur), Inequality.LE, 4.0)
/// >>> # CCC exposure <= 10%
/// >>> c2 = Constraint.tag_exposure_limit("ccc_limit", "rating", "CCC", 0.10)
/// >>> # IG exposure >= 50%
/// >>> c3 = Constraint.tag_exposure_minimum("ig_floor", "rating", "IG", 0.50)
/// >>> # Max turnover 20%
/// >>> c4 = Constraint.max_turnover("low_turnover", 0.20)
#[pyclass(
    name = "Constraint",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyConstraint {
    pub inner: Constraint,
}

#[pymethods]
impl PyConstraint {
    /// General metric bound, e.g. duration <= 4.0.
    #[staticmethod]
    #[pyo3(signature = (label, metric, op, rhs))]
    fn metric_bound(
        label: Option<String>,
        metric: PyMetricExpr,
        op: PyInequality,
        rhs: f64,
    ) -> Self {
        Self {
            inner: Constraint::MetricBound {
                label,
                metric: metric.inner,
                op: op.inner,
                rhs,
            },
        }
    }

    /// Tag exposure limit, e.g. rating=CCC weight <= 0.10.
    #[staticmethod]
    #[pyo3(signature = (label, tag_key, tag_value, max_share))]
    fn tag_exposure_limit(
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        max_share: f64,
    ) -> Self {
        Self {
            inner: Constraint::TagExposureLimit {
                label,
                tag_key,
                tag_value,
                max_share,
            },
        }
    }

    /// Minimum tag exposure, e.g. rating=IG weight >= 0.50.
    #[staticmethod]
    #[pyo3(signature = (label, tag_key, tag_value, min_share))]
    fn tag_exposure_minimum(
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        min_share: f64,
    ) -> Self {
        Self {
            inner: Constraint::TagExposureMinimum {
                label,
                tag_key,
                tag_value,
                min_share,
            },
        }
    }

    /// Weight bounds for all positions matching the filter.
    #[staticmethod]
    #[pyo3(signature = (label, filter, min, max))]
    fn weight_bounds(label: Option<String>, filter: PyPositionFilter, min: f64, max: f64) -> Self {
        Self {
            inner: Constraint::WeightBounds {
                label,
                filter: filter.inner,
                min,
                max,
            },
        }
    }

    /// Maximum turnover constraint.
    #[staticmethod]
    #[pyo3(signature = (label, max_turnover))]
    fn max_turnover(label: Option<String>, max_turnover: f64) -> Self {
        Self {
            inner: Constraint::MaxTurnover {
                label,
                max_turnover,
            },
        }
    }

    /// Maximum single position weight change.
    #[staticmethod]
    #[pyo3(signature = (label, filter, max_delta))]
    fn max_position_delta(label: Option<String>, filter: PyPositionFilter, max_delta: f64) -> Self {
        Self {
            inner: Constraint::MaxPositionDelta {
                label,
                filter: filter.inner,
                max_delta,
            },
        }
    }

    /// Budget/normalization constraint (usually sum w_i == 1.0).
    #[staticmethod]
    fn budget(rhs: f64) -> Self {
        Self {
            inner: Constraint::Budget { rhs },
        }
    }

    fn label(&self) -> Option<String> {
        self.inner.label().map(|s| s.to_string())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Trade Specification
// ===========================

/// Trade specification for a single position.
///
/// Attributes
/// ----------
/// position_id : str
///     Position identifier.
/// instrument_id : str
///     Underlying instrument identifier.
/// trade_type : TradeType
///     Type of trade (existing, new position, close-out).
/// current_quantity : float
///     Pre-trade quantity.
/// target_quantity : float
///     Post-trade quantity.
/// delta_quantity : float
///     Quantity change.
/// direction : TradeDirection
///     Buy/Sell/Hold classification.
/// current_weight : float
///     Pre-trade weight.
/// target_weight : float
///     Post-trade weight.
#[pyclass(
    name = "TradeSpec",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyTradeSpec {
    #[pyo3(get)]
    pub position_id: String,
    #[pyo3(get)]
    pub instrument_id: String,
    #[pyo3(get)]
    pub trade_type: PyTradeType,
    #[pyo3(get)]
    pub current_quantity: f64,
    #[pyo3(get)]
    pub target_quantity: f64,
    #[pyo3(get)]
    pub delta_quantity: f64,
    #[pyo3(get)]
    pub direction: PyTradeDirection,
    #[pyo3(get)]
    pub current_weight: f64,
    #[pyo3(get)]
    pub target_weight: f64,
}

impl From<&TradeSpec> for PyTradeSpec {
    fn from(spec: &TradeSpec) -> Self {
        Self {
            position_id: spec.position_id.to_string(),
            instrument_id: spec.instrument_id.clone(),
            trade_type: PyTradeType {
                inner: spec.trade_type,
            },
            current_quantity: spec.current_quantity,
            target_quantity: spec.target_quantity,
            delta_quantity: spec.delta_quantity,
            direction: PyTradeDirection {
                inner: spec.direction,
            },
            current_weight: spec.current_weight,
            target_weight: spec.target_weight,
        }
    }
}

#[pymethods]
impl PyTradeSpec {
    fn __repr__(&self) -> String {
        format!(
            "TradeSpec(position_id={}, direction={:?}, delta_quantity={:.4}, delta_weight={:.4})",
            self.position_id,
            self.direction.inner,
            self.delta_quantity,
            self.target_weight - self.current_weight
        )
    }
}

// ===========================
// Optimization Result
// ===========================

/// Solution of an optimization problem.
///
/// Attributes
/// ----------
/// optimal_weights : dict[str, float]
///     Optimal weights per position.
/// current_weights : dict[str, float]
///     Pre-trade weights.
/// weight_deltas : dict[str, float]
///     Weight changes (optimal - current).
/// objective_value : float
///     Objective value at solution.
/// status : OptimizationStatus
///     Optimization status.
///
/// Methods
/// -------
/// to_rebalanced_portfolio()
///     Generate new portfolio with adjusted quantities.
/// to_trade_list()
///     Generate trade list sorted by absolute quantity delta.
/// new_position_trades()
///     Get only trades for new positions (from candidates).
/// binding_constraints()
///     Get constraints that are binding at the solution (slack ≈ 0).
/// turnover()
///     Calculate total turnover (sum of absolute weight changes).
#[pyclass(
    name = "OptimizationResult",
    module = "finstack.portfolio.optimization"
)]
pub struct PyOptimizationResult {
    pub inner: PortfolioOptimizationResult,
}

#[pymethods]
impl PyOptimizationResult {
    #[getter]
    fn optimal_weights(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.optimal_weights)
    }

    #[getter]
    fn current_weights(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.current_weights)
    }

    #[getter]
    fn weight_deltas(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.weight_deltas)
    }

    #[getter]
    fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    #[getter]
    fn status(&self) -> PyOptimizationStatus {
        PyOptimizationStatus {
            inner: self.inner.status.clone(),
        }
    }

    #[getter]
    fn implied_quantities(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.implied_quantities)
    }

    #[getter]
    fn metric_values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.metric_values {
            dict.set_item(k.as_str(), *v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn dual_values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.dual_values {
            dict.set_item(k.as_str(), *v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn constraint_slacks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.constraint_slacks {
            dict.set_item(k.as_str(), *v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn meta(&self) -> PyResultsMeta {
        PyResultsMeta::new(self.inner.meta.clone())
    }

    #[getter]
    fn problem(&self) -> PyPortfolioOptimizationProblem {
        PyPortfolioOptimizationProblem {
            inner: self.inner.problem.clone(),
        }
    }

    /// Generate a new portfolio with quantities adjusted to target weights.
    fn to_rebalanced_portfolio(&self) -> PyResult<PyPortfolio> {
        let portfolio = self
            .inner
            .to_rebalanced_portfolio()
            .map_err(portfolio_to_py)?;
        Ok(PyPortfolio { inner: portfolio })
    }

    /// Generate trade list (delta from current to target).
    fn to_trade_list(&self) -> Vec<PyTradeSpec> {
        self.inner
            .to_trade_list()
            .iter()
            .map(PyTradeSpec::from)
            .collect()
    }

    /// Get only trades for new positions (from candidates).
    fn new_position_trades(&self) -> Vec<PyTradeSpec> {
        self.inner
            .new_position_trades()
            .iter()
            .map(PyTradeSpec::from)
            .collect()
    }

    /// Get binding constraints at the optimal solution (slack ≈ 0).
    fn binding_constraints(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (name, slack) in self.inner.binding_constraints() {
            dict.set_item(name, slack)?;
        }
        Ok(dict.into())
    }

    /// Calculate total turnover (sum of absolute weight changes).
    fn turnover(&self) -> f64 {
        self.inner.turnover()
    }

    fn __repr__(&self) -> String {
        format!(
            "OptimizationResult(status={:?}, objective_value={:.6}, turnover={:.4})",
            self.inner.status,
            self.inner.objective_value,
            self.turnover()
        )
    }
}

// ===========================
// Trade Universe
// ===========================

/// Candidate instrument that could be added to the portfolio.
///
/// Examples
/// --------
/// >>> from finstack import CandidatePosition, PositionUnit
/// >>> candidate = CandidatePosition.new("new_bond_1", "fund_1", bond_instrument, PositionUnit.FACE_VALUE)
/// >>> candidate = candidate.with_tag("rating", "BBB")
/// >>> candidate = candidate.with_max_weight(0.05)  # Max 5% of portfolio
#[pyclass(name = "CandidatePosition", module = "finstack.portfolio.optimization")]
pub struct PyCandidatePosition {
    pub inner: CandidatePosition,
}

#[pymethods]
impl PyCandidatePosition {
    #[staticmethod]
    fn new(
        id: String,
        entity_id: String,
        instrument: &Bound<'_, PyAny>,
        unit: PyPositionUnit,
    ) -> PyResult<Self> {
        let instrument_handle = extract_instrument(instrument)?;
        Ok(Self {
            inner: CandidatePosition::new(id, entity_id, instrument_handle.instrument, unit.inner),
        })
    }

    /// Add a tag to the candidate.
    fn with_tag(slf: PyRef<'_, Self>, key: String, value: String) -> Self {
        Self {
            inner: slf.inner.clone().with_tag(key, value),
        }
    }

    /// Set maximum weight for this candidate.
    fn with_max_weight(slf: PyRef<'_, Self>, max: f64) -> Self {
        Self {
            inner: slf.inner.clone().with_max_weight(max),
        }
    }

    /// Set minimum weight (if included) for this candidate.
    fn with_min_weight(slf: PyRef<'_, Self>, min: f64) -> Self {
        Self {
            inner: slf.inner.clone().with_min_weight(min),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Defines which instruments the optimizer can trade.
///
/// Examples
/// --------
/// >>> from finstack import TradeUniverse, PositionFilter
/// >>> # All positions tradeable
/// >>> universe = TradeUniverse.all_positions()
/// >>> # Only HY bonds tradeable
/// >>> universe = TradeUniverse.filtered(PositionFilter.by_tag("rating", "HY"))
/// >>> # Add candidate positions
/// >>> universe = universe.with_candidate(candidate1)
/// >>> universe = universe.with_candidates([candidate2, candidate3])
#[pyclass(
    name = "TradeUniverse",
    module = "finstack.portfolio.optimization",
    from_py_object
)]
#[derive(Clone)]
pub struct PyTradeUniverse {
    pub inner: TradeUniverse,
}

#[pymethods]
impl PyTradeUniverse {
    /// Create a universe where all existing positions are tradeable.
    #[staticmethod]
    fn all_positions() -> Self {
        Self {
            inner: TradeUniverse::all_positions(),
        }
    }

    /// Create a universe with only specific positions tradeable.
    #[staticmethod]
    fn filtered(filter: PyPositionFilter) -> Self {
        Self {
            inner: TradeUniverse::filtered(filter.inner),
        }
    }

    /// Add a candidate position to the universe.
    fn with_candidate(slf: PyRef<'_, Self>, candidate: &PyCandidatePosition) -> Self {
        Self {
            inner: slf.inner.clone().with_candidate(candidate.inner.clone()),
        }
    }

    /// Add multiple candidate positions.
    fn with_candidates(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
        candidates: Vec<Py<PyCandidatePosition>>,
    ) -> PyResult<Self> {
        let candidates_inner: Vec<CandidatePosition> = candidates
            .iter()
            .map(|c| c.borrow(py).inner.clone())
            .collect();
        Ok(Self {
            inner: slf.inner.clone().with_candidates(candidates_inner),
        })
    }

    /// Set positions to hold constant (not trade).
    fn with_held_positions(slf: PyRef<'_, Self>, filter: PyPositionFilter) -> Self {
        Self {
            inner: slf.inner.clone().with_held_positions(filter.inner),
        }
    }

    /// Allow short selling of candidate positions.
    fn allow_shorting_candidates(slf: PyRef<'_, Self>) -> Self {
        Self {
            inner: slf.inner.clone().allow_shorting_candidates(),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ===========================
// Optimization Problem
// ===========================

/// Complete optimization problem specification.
///
/// Examples
/// --------
/// >>> from finstack import PortfolioOptimizationProblem, Objective, MetricExpr, PerPositionMetric, MetricId, Constraint
/// >>> # Create objective
/// >>> ytm = PerPositionMetric.metric(MetricId.YTM)
/// >>> obj = Objective.maximize(MetricExpr.value_weighted_average(ytm))
/// >>> # Create problem
/// >>> problem = PortfolioOptimizationProblem.new(portfolio, obj)
/// >>> # Add constraints
/// >>> problem = problem.with_constraint(Constraint.tag_exposure_limit(None, "rating", "CCC", 0.10))
/// >>> problem = problem.with_constraint(Constraint.max_turnover(None, 0.20))
/// >>> # Solve
/// >>> result = problem.optimize(market_context)
#[pyclass(
    name = "PortfolioOptimizationProblem",
    module = "finstack.portfolio.optimization"
)]
pub struct PyPortfolioOptimizationProblem {
    pub inner: PortfolioOptimizationProblem,
}

#[pymethods]
impl PyPortfolioOptimizationProblem {
    /// Create a basic problem optimizing all positions in the portfolio.
    #[staticmethod]
    fn new(portfolio: &Bound<'_, PyAny>, objective: PyObjective) -> PyResult<Self> {
        let portfolio_inner = extract_portfolio(portfolio)?;
        Ok(Self {
            inner: PortfolioOptimizationProblem::new(portfolio_inner, objective.inner),
        })
    }

    /// Set the trade universe.
    fn with_trade_universe(slf: PyRef<'_, Self>, universe: PyTradeUniverse) -> Self {
        Self {
            inner: slf.inner.clone().with_trade_universe(universe.inner),
        }
    }

    /// Add a single constraint.
    fn with_constraint(slf: PyRef<'_, Self>, constraint: PyConstraint) -> Self {
        Self {
            inner: slf.inner.clone().with_constraint(constraint.inner),
        }
    }

    /// Add multiple constraints.
    fn with_constraints(slf: PyRef<'_, Self>, constraints: Vec<PyConstraint>) -> Self {
        let constraints_inner = constraints.into_iter().map(|c| c.inner).collect::<Vec<_>>();
        Self {
            inner: slf.inner.clone().with_constraints(constraints_inner),
        }
    }

    /// Set weighting scheme.
    #[setter]
    fn set_weighting(&mut self, weighting: PyWeightingScheme) {
        self.inner.weighting = weighting.inner;
    }

    /// Set missing metric policy.
    #[setter]
    fn set_missing_metric_policy(&mut self, policy: PyMissingMetricPolicy) {
        self.inner.missing_metric_policy = policy.inner;
    }

    /// Set problem label.
    #[setter]
    fn set_label(&mut self, label: Option<String>) {
        self.inner.label = label;
    }

    #[getter]
    fn get_label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    #[getter]
    fn get_weighting(&self) -> PyWeightingScheme {
        PyWeightingScheme {
            inner: self.inner.weighting,
        }
    }

    #[getter]
    fn get_missing_metric_policy(&self) -> PyMissingMetricPolicy {
        PyMissingMetricPolicy {
            inner: self.inner.missing_metric_policy,
        }
    }

    #[getter]
    fn constraints(&self) -> Vec<PyConstraint> {
        self.inner
            .constraints
            .iter()
            .map(|c| PyConstraint { inner: c.clone() })
            .collect()
    }

    #[getter]
    fn portfolio(&self) -> PyPortfolio {
        PyPortfolio::new(self.inner.portfolio.clone())
    }

    /// Optimize the problem.
    #[pyo3(signature = (market_context, config=None))]
    fn optimize(
        &self,
        market_context: &Bound<'_, PyAny>,
        config: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyOptimizationResult> {
        let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;
        let cfg = extract_config_or_default(config)?;

        let optimizer = DefaultLpOptimizer::default();
        let result = optimizer
            .optimize(&self.inner, &market_ctx.inner, &cfg)
            .map_err(portfolio_to_py)?;

        Ok(PyOptimizationResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioOptimizationProblem(label={:?}, constraints={})",
            self.inner.label,
            self.inner.constraints.len()
        )
    }
}

// ===========================
// MaxYieldWithCccLimitResult
// ===========================

/// Typed result from `optimize_max_yield_with_ccc_limit`.
#[pyclass(
    name = "MaxYieldWithCccLimitResult",
    module = "finstack.portfolio.optimization"
)]
pub struct PyMaxYieldWithCccLimitResult {
    pub inner: MaxYieldWithCccLimitResult,
}

#[pymethods]
impl PyMaxYieldWithCccLimitResult {
    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    #[getter]
    fn status(&self) -> PyOptimizationStatus {
        PyOptimizationStatus {
            inner: self.inner.status.clone(),
        }
    }

    #[getter]
    fn status_label(&self) -> String {
        self.inner.status_label.clone()
    }

    #[getter]
    fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    #[getter]
    fn ccc_weight(&self) -> f64 {
        self.inner.ccc_weight
    }

    #[getter]
    fn optimal_weights(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.optimal_weights)
    }

    #[getter]
    fn current_weights(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.current_weights)
    }

    #[getter]
    fn weight_deltas(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        map_weights(py, &self.inner.weight_deltas)
    }

    fn __repr__(&self) -> String {
        format!(
            "MaxYieldWithCccLimitResult(status={}, objective={:.6}, ccc_weight={:.4})",
            self.inner.status_label, self.inner.objective_value, self.inner.ccc_weight
        )
    }
}

// ===========================
// DefaultLpOptimizer
// ===========================

/// Default LP optimizer for portfolio optimization.
///
/// Examples
/// --------
/// >>> from finstack.portfolio.optimization import DefaultLpOptimizer
/// >>> opt = DefaultLpOptimizer()
/// >>> opt = DefaultLpOptimizer(tolerance=1e-6, max_iterations=5000)
/// >>> result = opt.optimize(problem, market_context)
#[pyclass(
    name = "DefaultLpOptimizer",
    module = "finstack.portfolio.optimization"
)]
pub struct PyDefaultLpOptimizer {
    pub inner: DefaultLpOptimizer,
}

#[pymethods]
impl PyDefaultLpOptimizer {
    #[new]
    #[pyo3(signature = (tolerance=1.0e-8, max_iterations=10_000))]
    fn new(tolerance: f64, max_iterations: usize) -> Self {
        Self {
            inner: DefaultLpOptimizer {
                tolerance,
                max_iterations,
            },
        }
    }

    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[pyo3(signature = (problem, market_context, config=None))]
    fn optimize(
        &self,
        problem: &PyPortfolioOptimizationProblem,
        market_context: &Bound<'_, PyAny>,
        config: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyOptimizationResult> {
        let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;
        let cfg = extract_config_or_default(config)?;

        let result = self
            .inner
            .optimize(&problem.inner, &market_ctx.inner, &cfg)
            .map_err(portfolio_to_py)?;
        Ok(PyOptimizationResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!(
            "DefaultLpOptimizer(tolerance={}, max_iterations={})",
            self.inner.tolerance, self.inner.max_iterations
        )
    }
}

/// Register optimization helpers.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    // Register helper function
    let func = wrap_pyfunction!(py_optimize_max_yield_with_ccc_limit, parent)?;
    parent.add_function(func)?;

    // Register all classes
    parent.add_class::<PyWeightingScheme>()?;
    parent.add_class::<PyMissingMetricPolicy>()?;
    parent.add_class::<PyInequality>()?;
    parent.add_class::<PyOptimizationStatus>()?;
    parent.add_class::<PyTradeDirection>()?;
    parent.add_class::<PyTradeType>()?;
    parent.add_class::<PyPerPositionMetric>()?;
    parent.add_class::<PyMetricExpr>()?;
    parent.add_class::<PyObjective>()?;
    parent.add_class::<PyPositionFilter>()?;
    parent.add_class::<PyConstraint>()?;
    parent.add_class::<PyTradeSpec>()?;
    parent.add_class::<PyOptimizationResult>()?;
    parent.add_class::<PyCandidatePosition>()?;
    parent.add_class::<PyTradeUniverse>()?;
    parent.add_class::<PyPortfolioOptimizationProblem>()?;
    parent.add_class::<PyMaxYieldWithCccLimitResult>()?;
    parent.add_class::<PyDefaultLpOptimizer>()?;

    Ok(vec![
        "optimize_max_yield_with_ccc_limit".to_string(),
        "WeightingScheme".to_string(),
        "MissingMetricPolicy".to_string(),
        "Inequality".to_string(),
        "OptimizationStatus".to_string(),
        "TradeDirection".to_string(),
        "TradeType".to_string(),
        "PerPositionMetric".to_string(),
        "MetricExpr".to_string(),
        "Objective".to_string(),
        "PositionFilter".to_string(),
        "Constraint".to_string(),
        "TradeSpec".to_string(),
        "OptimizationResult".to_string(),
        "CandidatePosition".to_string(),
        "TradeUniverse".to_string(),
        "PortfolioOptimizationProblem".to_string(),
        "MaxYieldWithCccLimitResult".to_string(),
        "DefaultLpOptimizer".to_string(),
    ])
}
