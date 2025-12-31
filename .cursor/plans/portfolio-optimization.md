## Portfolio Optimization – Design Plan

### 1. Goals and Scope

- **Primary goal**: Add a generic, deterministic portfolio optimization facility on top of the existing `portfolio`+`valuations` stack that:
  - **Optimizes portfolio weights** over an existing `Portfolio` (per-position decision variables).
  - **Expresses objectives and constraints** in terms of portfolio-level metrics derived from valuations and tags.
  - **Respects finstack invariants**: determinism, currency safety, stable serde, and reusable rounding context.
- **Initial scope (MVP)**:
  - Linear (or affine) **objectives** and **constraints** in the position weights.
  - Support for **value-weighted** portfolios (weights sum to 1 in base currency PV).
  - Constraints on **duration-like metrics**, **exposures by tag** (e.g. rating buckets), **turnover**, and basic **weight bounds**.
  - **Rebalancing output**: generate trade lists and rebalanced portfolios from optimization results.
  - Integration at the Rust level (`finstack::portfolio`) with a clean path for Python bindings later.
- **Non-goals (for this iteration)**:
  - Non-linear objectives (e.g. variance-minimization, quadratic tracking error).
  - Scenario-aware or time-series-aware optimization (e.g. robust worst-case constraints).
  - Transaction cost modelling, integer constraints, or path-dependent constraints.

The design should make it easy to add these later without breaking existing APIs.

---

### 2. Existing Architecture Overview (Relevant Pieces)

- **`portfolio::Portfolio` (`src/portfolio.rs`)**
  - Holds `entities`, a flat `Vec<Position>`, `base_ccy`, `as_of`, and portfolio-level `tags`/`meta`.
  - Has helpers like `positions_for_entity`, `positions_with_tag`, and `validate`.
- **`portfolio::valuation` (`src/valuation.rs`)**
  - `PositionValue` – stores `position_id`, `entity_id`, `value_native`, `value_base`, and an optional `ValuationResult`.
  - `PortfolioValuation` – maps `position_id → PositionValue`, and aggregates to `by_entity` and `total_base_ccy`.
  - `value_portfolio_with_options` – prices all positions (serial or parallel), computing a **fixed set of "standard" metrics** via `standard_portfolio_metrics() -> Vec<MetricId>`.
- **`portfolio::metrics` (`src/metrics.rs`)**
  - `PortfolioMetrics` – aggregates summable metrics across positions using `ValuationResult::measures`.
  - `aggregate_metrics` – serial/parallel aggregation using Neumaier summation and `is_summable(metric_id: &str)`.
- **`portfolio::results` (`src/results.rs`)**
  - `PortfolioResults` – wraps `PortfolioValuation`, `PortfolioMetrics`, and a `ResultsMeta` with numeric mode, parallel flag, etc.
- **`valuations::metrics` (`finstack/valuations/src/metrics`)**
  - `MetricId` – `Cow<'static, str>`-backed identifier for all instrument-level metrics.
  - Standard metrics include:
    - `MetricId::DurationMac` ("duration_mac") – Macaulay duration
    - `MetricId::DurationMod` ("duration_mod") – Modified duration
    - `MetricId::Ytm` ("ytm") – Yield to maturity
    - `MetricId::Dv01` ("dv01") – Dollar value of 01
    - `MetricId::WAL` ("wal") – Weighted average life
  - `ValuationResult` – holds `value: Money`, `measures: IndexMap<String, f64>`, bucketed series, and metadata.

**Key architectural decision**: portfolio optimization will be implemented **entirely on top of these valuation results**, never bypassing the valuation engine for cashflow or metric computation. The optimizer merely:

1. Requests a set of per-position metrics via `price_with_metrics`.
2. Converts those metrics + tags into linear coefficients.
3. Solves for the weights.

---

### 3. Module Layout and Public API Surface

- **New module** in `finstack/portfolio`:

  - File: `finstack/portfolio/src/optimization.rs` (or `optimization/` directory for larger scope)
  - Export from `lib.rs`:

    ```rust
    pub mod optimization;

    pub use optimization::{
        // Problem and result types
        PortfolioOptimizationProblem,
        PortfolioOptimizationResult,
        PortfolioOptimizer,
        DefaultLpOptimizer,

        // Trade universe types
        TradeUniverse,
        CandidatePosition,
        PositionFilter,
        WeightingScheme,

        // Metric and expression types
        MetricExpr,
        PerPositionMetric,
        MissingMetricPolicy,

        // Objective and constraint types
        Objective,
        Constraint,
        Inequality,

        // Output types
        TradeSpec,
        TradeDirection,
        TradeType,
    };
    ```

- **No changes to serde schemas** in this iteration:
  - Optimization types start as **non-serialized Rust types**.
  - A later Python/WASM phase can add `*_Spec` serde mirrors if/when needed.

---

### 4. Core Domain Types

#### 4.1 Weighting scheme and trade universe

We define what each decision variable represents and which instruments are tradeable.

```rust
/// How optimization weights are defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightingScheme {
    /// w_i is share of portfolio base_ccy PV; sum_i w_i = 1.
    ValueWeight,

    /// w_i is share of some notional exposure; still normalized to sum_i w_i = 1.
    NotionalWeight,

    /// w_i scales the current quantity (e.g. units or face value).
    /// For MVP we can treat w_i as a fraction of current position size.
    UnitScaling,
}

/// Filters for selecting which positions are decision variables.
#[derive(Clone, Debug)]
pub enum PositionFilter {
    /// All positions in the portfolio.
    All,

    /// Filter by entity ID.
    ByEntityId(crate::types::EntityId),

    /// Filter by tag key/value (e.g. rating = "HY").
    ByTag { key: String, value: String },

    /// Filter by multiple position IDs.
    ByPositionIds(Vec<crate::types::PositionId>),

    /// Exclude positions matching the inner filter.
    Not(Box<PositionFilter>),
}
```

---

#### 4.2 Trade Universe: Candidates and Restrictions

The **Trade Universe** defines which instruments the optimizer can trade. This is separate from the portfolio itself and enables:

- **Hedging trades**: Add instruments not currently held (e.g., CDS hedges, futures overlays)
- **What-if analysis**: Evaluate potential new positions alongside existing holdings
- **Restricted trading**: Lock certain positions while allowing others to be adjusted

```rust
use finstack_valuations::instruments::common::traits::Instrument;
use std::sync::Arc;

/// A candidate instrument that could be added to the portfolio.
///
/// This represents an instrument not currently held but available for trading.
/// The optimizer can allocate weight to candidates (up to max_weight).
#[derive(Clone, Debug)]
pub struct CandidatePosition {
    /// Unique identifier for this candidate (becomes position_id if traded).
    pub id: crate::types::PositionId,

    /// Entity that would own this position.
    pub entity_id: crate::types::EntityId,

    /// The instrument that could be traded.
    pub instrument: Arc<dyn Instrument>,

    /// Unit type for quantity interpretation.
    pub unit: crate::position::PositionUnit,

    /// Tags for the candidate (used in constraints like TagExposureLimit).
    pub tags: indexmap::IndexMap<String, String>,

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
        id: impl Into<crate::types::PositionId>,
        entity_id: impl Into<crate::types::EntityId>,
        instrument: Arc<dyn Instrument>,
        unit: crate::position::PositionUnit,
    ) -> Self {
        Self {
            id: id.into(),
            entity_id: entity_id.into(),
            instrument,
            unit,
            tags: indexmap::IndexMap::new(),
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
/// 1. **Tradeable positions**: Existing portfolio positions that can be adjusted
/// 2. **Held positions**: Existing positions locked at current weight (excluded from optimization)
/// 3. **Candidate positions**: New instruments that could be added to the portfolio
///
/// # Examples
///
/// ## Default: Trade entire portfolio
/// ```rust,ignore
/// let universe = TradeUniverse::default(); // All existing positions tradeable
/// ```
///
/// ## Add hedging candidates
/// ```rust,ignore
/// let universe = TradeUniverse::default()
///     .with_candidate(CandidatePosition::new(
///         "HEDGE_CDS_IG",
///         DUMMY_ENTITY_ID,
///         Arc::new(cds_index),
///         PositionUnit::Notional(Some(Currency::USD)),
///     ).with_tag("type", "hedge").with_max_weight(0.10));
/// ```
///
/// ## Restrict trading to subset of portfolio
/// ```rust,ignore
/// let universe = TradeUniverse::default()
///     .with_held_positions(PositionFilter::ByTag {
///         key: "locked".into(),
///         value: "true".into()
///     });
/// ```
#[derive(Clone, Debug, Default)]
pub struct TradeUniverse {
    /// Filter for existing positions that can be traded.
    /// Positions matching this filter have their weights optimized.
    /// Default: All positions are tradeable.
    pub tradeable_filter: PositionFilter,

    /// Filter for existing positions that are held constant.
    /// Positions matching this filter keep their current weight.
    /// Takes precedence over tradeable_filter if both match.
    pub held_filter: Option<PositionFilter>,

    /// Candidate instruments not currently in the portfolio.
    /// These start with weight 0 and can be added by the optimizer.
    pub candidates: Vec<CandidatePosition>,

    /// Whether candidates can receive negative weights (short selling).
    /// Default: false (long-only for new positions).
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
            ..Default::default()
        }
    }

    /// Add a candidate position to the universe.
    pub fn with_candidate(mut self, candidate: CandidatePosition) -> Self {
        self.candidates.push(candidate);
        self
    }

    /// Add multiple candidate positions.
    pub fn with_candidates(mut self, candidates: impl IntoIterator<Item = CandidatePosition>) -> Self {
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

    /// Check if a position is tradeable.
    pub fn is_tradeable(&self, position: &crate::position::Position) -> bool {
        // Check held filter first (takes precedence)
        if let Some(ref held) = self.held_filter {
            if matches_filter(position, held) {
                return false;
            }
        }
        // Then check tradeable filter
        matches_filter(position, &self.tradeable_filter)
    }
}

/// Check if a position matches a filter.
fn matches_filter(position: &crate::position::Position, filter: &PositionFilter) -> bool {
    match filter {
        PositionFilter::All => true,
        PositionFilter::ByEntityId(entity_id) => position.entity_id == *entity_id,
        PositionFilter::ByTag { key, value } => {
            position.tags.get(key).map(|v| v == value).unwrap_or(false)
        }
        PositionFilter::ByPositionIds(ids) => ids.contains(&position.position_id),
        PositionFilter::Not(inner) => !matches_filter(position, inner),
    }
}
```

**Use cases**:

1. **Basic optimization** (existing behavior):
   ```rust
   let universe = TradeUniverse::all_positions();
   ```

2. **Add hedging instruments**:
   ```rust
   let universe = TradeUniverse::all_positions()
       .with_candidate(CandidatePosition::new(
           "SP500_PUT",
           DUMMY_ENTITY_ID,
           Arc::new(sp500_put_option),
           PositionUnit::Notional(Some(Currency::USD)),
       ).with_tag("type", "hedge").with_max_weight(0.05))
       .with_candidate(CandidatePosition::new(
           "IG_CDS_INDEX",
           DUMMY_ENTITY_ID,
           Arc::new(cdx_ig),
           PositionUnit::Notional(Some(Currency::USD)),
       ).with_tag("type", "hedge").with_max_weight(0.10));
   ```

3. **What-if: Evaluate adding new bonds**:
   ```rust
   let universe = TradeUniverse::all_positions()
       .with_candidates(new_bond_candidates.iter().map(|bond| {
           CandidatePosition::new(
               bond.id().to_string(),
               "FUND_A",
               Arc::new(bond.clone()),
               PositionUnit::FaceValue,
           ).with_max_weight(0.02)  // Max 2% in any single new bond
       }));
   ```

4. **Lock illiquid positions**:
   ```rust
   let universe = TradeUniverse::all_positions()
       .with_held_positions(PositionFilter::ByTag {
           key: "liquidity".into(),
           value: "low".into(),
       });
   ```

5. **Only trade specific positions**:
   ```rust
   let universe = TradeUniverse::filtered(PositionFilter::ByTag {
       key: "tradeable".into(),
       value: "true".into(),
   });
   ```

**Implementation notes**:

- Internally, we will **materialize a deterministic ordered list** of decision variables:
  - First: tradeable existing positions (from `portfolio.positions`, filtered)
  - Then: candidate positions (in order they were added)
  - Map each to an index `i` in `[0, n_decisions)`
- Held positions contribute to portfolio metrics but are not decision variables
- Candidates start with weight 0 and current_quantity 0

---

#### 4.3 Per-position metrics (features)

We need a generic way to map each position to a scalar metric value \( m_i \) that can be used in constraints and objectives.

```rust
use finstack_valuations::metrics::MetricId;

/// Where a per-position scalar metric comes from.
#[derive(Clone, Debug)]
pub enum PerPositionMetric {
    /// Directly from ValuationResult::measures using a standard MetricId.
    /// Preferred for compile-time checked metrics.
    ///
    /// Examples:
    /// - `Metric(MetricId::DurationMod)` for modified duration
    /// - `Metric(MetricId::Ytm)` for yield to maturity
    /// - `Metric(MetricId::Dv01)` for DV01
    Metric(MetricId),

    /// From ValuationResult::measures using a string key (for custom or
    /// bucketed metrics stored by name).
    CustomKey(String),

    /// Use the base_ccy PV of the position (after scaling).
    PvBase,

    /// Use the native-currency PV of the position (after scaling).
    PvNative,

    /// Tag-based 0/1 indicator: 1.0 if tag matches, else 0.0.
    TagEquals { key: String, value: String },

    /// Constant scalar for all positions.
    Constant(f64),
}
```

**Metric key mapping**:

The actual metric keys stored in `ValuationResult::measures` follow snake_case naming:
- `MetricId::DurationMac` → `"duration_mac"`
- `MetricId::DurationMod` → `"duration_mod"`
- `MetricId::Ytm` → `"ytm"`
- `MetricId::Dv01` → `"dv01"`
- `MetricId::WAL` → `"wal"`

When using `PerPositionMetric::Metric(id)`, the optimizer will call `id.as_str()` to look up the value.

---

#### 4.4 Missing metric policy

Not all positions have all metrics (e.g., cash positions don't have duration). We need a policy for handling this:

```rust
/// How to handle missing metrics for a position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissingMetricPolicy {
    /// Treat missing as 0.0 (default, appropriate for duration-like metrics).
    #[default]
    Zero,

    /// Exclude position from constraint evaluation (position keeps current weight).
    Exclude,

    /// Fail with error if any required metric is missing.
    Strict,
}
```

---

#### 4.5 Portfolio metric expressions

Optimization operates on **portfolio-level metrics** expressed as linear forms of weights. We model these as:

```rust
/// Portfolio-level scalar metric expressed in terms of position metrics + weights.
#[derive(Clone, Debug)]
pub enum MetricExpr {
    /// Sum_i w_i * m_i, where m_i comes from a PerPositionMetric.
    WeightedSum { metric: PerPositionMetric },

    /// Value-weighted average: sum_i w_i * m_i, with implicit sum_i w_i == 1.
    /// This is appropriate for duration or yield when weights are ValueWeight.
    ValueWeightedAverage { metric: PerPositionMetric },

    /// Exposure share for a tag bucket: sum_i w_i * I[tag == value].
    /// Assumes weights are already normalized (e.g. ValueWeight).
    TagExposureShare {
        tag_key: String,
        tag_value: String,
    },
}
```

**Lowering to linear forms**:

- Each `MetricExpr` is lowered to a vector of coefficients `a_i` such that:
  - `WeightedSum`: \( \sum_i a_i w_i \) with `a_i = m_i`.
  - `ValueWeightedAverage`: \( \sum_i a_i w_i \) with `a_i = m_i` and the **budget constraint** `∑ w_i = 1` enforces the normalization.
  - `TagExposureShare`: `a_i = 1.0` if `tag == value`, else `0.0`, again with `∑ w_i = 1`.

We keep this layer **purely linear** so it can feed standard LP/QP solvers.

---

#### 4.6 Objective and constraints

```rust
/// Optimization direction and target.
#[derive(Clone, Debug)]
pub enum Objective {
    Maximize(MetricExpr),
    Minimize(MetricExpr),
}

/// Inequality/equality operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inequality {
    Le,
    Ge,
    Eq,
}

/// Declarative constraint specification.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// General metric bound, e.g. duration <= 4.0.
    MetricBound {
        /// Human-readable label for debugging/diagnostics.
        label: Option<String>,
        metric: MetricExpr,
        op: Inequality,
        rhs: f64,
    },

    /// Tag exposure limit, e.g. rating=CCC weight <= 0.10.
    TagExposureLimit {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        max_share: f64, // in [0, 1]
    },

    /// Minimum tag exposure, e.g. rating=IG weight >= 0.50.
    TagExposureMinimum {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        min_share: f64, // in [0, 1]
    },

    /// Weight bounds for all positions matching the filter.
    WeightBounds {
        label: Option<String>,
        filter: PositionFilter,
        min: f64, // inclusive
        max: f64, // inclusive
    },

    /// Maximum turnover constraint: Σ |w_new - w_current| ≤ max_turnover.
    MaxTurnover {
        label: Option<String>,
        max_turnover: f64,
    },

    /// Maximum single position weight change: |w_new - w_current| ≤ max_delta.
    MaxPositionDelta {
        label: Option<String>,
        filter: PositionFilter,
        max_delta: f64,
    },

    /// Budget/normalization constraint: usually ∑ w_i == 1.0.
    Budget { rhs: f64 },
}

impl Constraint {
    /// Get the constraint label (for diagnostics).
    pub fn label(&self) -> Option<&str> {
        match self {
            Constraint::MetricBound { label, .. } => label.as_deref(),
            Constraint::TagExposureLimit { label, .. } => label.as_deref(),
            Constraint::TagExposureMinimum { label, .. } => label.as_deref(),
            Constraint::WeightBounds { label, .. } => label.as_deref(),
            Constraint::MaxTurnover { label, .. } => label.as_deref(),
            Constraint::MaxPositionDelta { label, .. } => label.as_deref(),
            Constraint::Budget { .. } => Some("budget"),
        }
    }
}
```

**Examples**:

- Duration constraint:

  ```rust
  Constraint::MetricBound {
      label: Some("duration_limit".into()),
      metric: MetricExpr::ValueWeightedAverage {
          metric: PerPositionMetric::Metric(MetricId::DurationMod),
      },
      op: Inequality::Le,
      rhs: 4.0,
  }
  ```

- CCC exposure constraint:

  ```rust
  Constraint::TagExposureLimit {
      label: Some("ccc_limit".into()),
      tag_key: "rating".into(),
      tag_value: "CCC".into(),
      max_share: 0.10,
  }
  ```

- Turnover constraint (max 20% portfolio turnover):

  ```rust
  Constraint::MaxTurnover {
      label: Some("turnover_limit".into()),
      max_turnover: 0.20,
  }
  ```

---

#### 4.7 Problem and result types

```rust
use finstack_core::config::ResultsMeta;

/// Complete optimization problem specification.
#[derive(Clone, Debug)]
pub struct PortfolioOptimizationProblem {
    /// The existing portfolio to optimize.
    pub portfolio: crate::Portfolio,

    /// How weights are defined (value-weighted, notional, etc.).
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

    /// Optional label/metadata for auditability.
    pub label: Option<String>,
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

impl PortfolioOptimizationProblem {
    /// Create a basic problem optimizing all positions in the portfolio.
    pub fn new(
        portfolio: crate::Portfolio,
        objective: Objective,
    ) -> Self {
        Self {
            portfolio,
            weighting: WeightingScheme::ValueWeight,
            trade_universe: TradeUniverse::default(),
            objective,
            constraints: vec![Constraint::Budget { rhs: 1.0 }],
            missing_metric_policy: MissingMetricPolicy::Zero,
            label: None,
            meta: indexmap::IndexMap::new(),
        }
    }

    /// Set the trade universe.
    pub fn with_trade_universe(mut self, universe: TradeUniverse) -> Self {
        self.trade_universe = universe;
        self
    }

    /// Add a constraint.
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add multiple constraints.
    pub fn with_constraints(mut self, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        self.constraints.extend(constraints);
        self
    }
}

/// Status of an optimization run.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn is_feasible(&self) -> bool {
        matches!(self, Self::Optimal | Self::FeasibleButSuboptimal)
    }
}

/// Solution of an optimization problem.
#[derive(Clone, Debug)]
pub struct PortfolioOptimizationResult {
    /// Echo of the original problem for traceability.
    pub problem: PortfolioOptimizationProblem,

    /// Pre-trade weights (current portfolio state).
    pub current_weights: indexmap::IndexMap<crate::types::PositionId, f64>,

    /// Optimal weights per position (for positions in the universe).
    /// Non-universe positions implicitly keep weight = current_weight.
    pub optimal_weights: indexmap::IndexMap<crate::types::PositionId, f64>,

    /// Weight changes (optimal - current).
    pub weight_deltas: indexmap::IndexMap<crate::types::PositionId, f64>,

    /// Implied target quantities for each position (units / face / notional).
    pub implied_quantities: indexmap::IndexMap<crate::types::PositionId, f64>,

    /// Objective value at the solution.
    pub objective_value: f64,

    /// Evaluated portfolio-level metrics at the solution (for key MetricExprs).
    pub metric_values: indexmap::IndexMap<String, f64>,

    /// Optimization status.
    pub status: OptimizationStatus,

    /// Shadow prices / dual values for constraints (how much objective improves
    /// per unit relaxation). Key is constraint label or auto-generated identifier.
    pub dual_values: indexmap::IndexMap<String, f64>,

    /// Constraint slack values (positive = slack, zero = binding).
    pub constraint_slacks: indexmap::IndexMap<String, f64>,

    /// Calculation metadata (numeric mode, rounding context, timing).
    pub meta: ResultsMeta,
}
```

---

#### 4.8 Trade list and rebalancing helpers

Essential outputs for practical use:

```rust
/// Direction of a trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeDirection {
    Buy,
    Sell,
    Hold,
}

/// Whether a trade is for an existing position or a new candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeType {
    /// Adjusting an existing portfolio position.
    Existing,
    /// Adding a new position from the candidate universe.
    NewPosition,
    /// Closing out an existing position entirely.
    CloseOut,
}

/// Trade specification for a single position.
#[derive(Clone, Debug)]
pub struct TradeSpec {
    pub position_id: crate::types::PositionId,
    pub instrument_id: String,
    pub trade_type: TradeType,
    pub current_quantity: f64,
    pub target_quantity: f64,
    pub delta_quantity: f64,  // target - current
    pub direction: TradeDirection,
    pub current_weight: f64,
    pub target_weight: f64,
}

impl PortfolioOptimizationResult {
    /// Generate a new portfolio with quantities adjusted to target weights.
    ///
    /// This is the primary output most users need - a portfolio they can
    /// actually use for order generation.
    ///
    /// # Errors
    ///
    /// Returns error if the optimization did not find a feasible solution.
    pub fn to_rebalanced_portfolio(&self) -> crate::Result<crate::Portfolio> {
        if !self.status.is_feasible() {
            return Err(crate::PortfolioError::invalid_input(
                "Cannot generate rebalanced portfolio from infeasible solution"
            ));
        }
        // Implementation: clone portfolio, update position quantities
        todo!()
    }

    /// Generate trade list (delta from current to target).
    ///
    /// Returns trades sorted by absolute delta (largest first).
    /// Includes both adjustments to existing positions and new positions from candidates.
    pub fn to_trade_list(&self) -> Vec<TradeSpec> {
        let mut trades: Vec<TradeSpec> = self.optimal_weights.iter()
            .filter_map(|(pos_id, &target_weight)| {
                let current_weight = self.current_weights.get(pos_id).copied().unwrap_or(0.0);
                let delta = target_weight - current_weight;

                // Skip positions with negligible change
                if delta.abs() < 1e-9 {
                    return None;
                }

                // Check if this is an existing position or a candidate
                let existing_position = self.problem.portfolio.get_position(pos_id.as_str());
                let is_candidate = existing_position.is_none();

                let current_qty = existing_position.map(|p| p.quantity).unwrap_or(0.0);
                let target_qty = self.implied_quantities.get(pos_id).copied().unwrap_or(0.0);

                // Determine trade type
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
                        self.problem.trade_universe.candidates.iter()
                            .find(|c| c.id == *pos_id)
                            .map(|c| c.instrument.id().to_string())
                    })
                    .unwrap_or_default();

                Some(TradeSpec {
                    position_id: pos_id.clone(),
                    instrument_id,
                    trade_type,
                    current_quantity: current_qty,
                    target_quantity: target_qty,
                    delta_quantity: target_qty - current_qty,
                    direction: if delta > 0.0 { TradeDirection::Buy }
                              else if delta < 0.0 { TradeDirection::Sell }
                              else { TradeDirection::Hold },
                    current_weight,
                    target_weight,
                })
            })
            .collect();

        // Sort by absolute delta (largest trades first)
        trades.sort_by(|a, b| b.delta_quantity.abs().partial_cmp(&a.delta_quantity.abs())
            .unwrap_or(std::cmp::Ordering::Equal));

        trades
    }

    /// Get only trades for new positions (from candidates).
    pub fn new_position_trades(&self) -> Vec<&TradeSpec> {
        self.to_trade_list().iter()
            .filter(|t| t.trade_type == TradeType::NewPosition)
            .collect()
    }

    /// Get binding constraints at the optimal solution (slack ≈ 0).
    pub fn binding_constraints(&self) -> Vec<(&str, f64)> {
        self.constraint_slacks.iter()
            .filter(|(_, &slack)| slack.abs() < 1e-6)
            .map(|(name, &slack)| (name.as_str(), slack))
            .collect()
    }

    /// Calculate total turnover (sum of absolute weight changes).
    pub fn turnover(&self) -> f64 {
        self.weight_deltas.values().map(|d| d.abs()).sum()
    }
}
```

---

#### 4.9 Optimizer trait and default implementation

```rust
/// Optimizer interface; allows swapping implementations (LP, QP, etc.).
pub trait PortfolioOptimizer: Send + Sync {
    fn optimize(
        &self,
        problem: &PortfolioOptimizationProblem,
        market: &MarketContext,
        config: &FinstackConfig,
    ) -> crate::Result<PortfolioOptimizationResult>;
}
```

For the MVP we introduce a **single concrete implementation**:

```rust
/// LP-based optimizer using the good_lp crate as backend.
pub struct DefaultLpOptimizer {
    /// Solver tolerance for optimality.
    pub tolerance: f64,
    /// Maximum iterations.
    pub max_iterations: usize,
}

impl Default for DefaultLpOptimizer {
    fn default() -> Self {
        Self {
            tolerance: 1e-8,
            max_iterations: 10_000,
        }
    }
}
```

**Backend choice**: `good_lp` crate

We use the [`good_lp`](https://crates.io/crates/good_lp) crate which provides:

- **Solver abstraction**: Unified API over multiple backends (highs, coin_cbc, lpsolve, minilp).
- **Pure Rust option**: Can use `minilp` backend for no C dependencies.
- **Deterministic**: Supports single-threaded, deterministic solvers.
- **Actively maintained**: Regular updates and good documentation.
- **Feature flags**: Different backends can be enabled via Cargo features.

Configuration in `Cargo.toml`:

```toml
[dependencies]
good_lp = { version = "1.8", default-features = false }

[features]
default = ["optimization"]
optimization = ["good_lp/minilp"]  # Pure Rust, no unsafe
optimization-highs = ["good_lp/highs"]  # Faster, requires HiGHS library
```

---

### 5. Valuation and Metric Requirements

#### 5.1 Extending `PortfolioValuationOptions` to control metrics

Currently, `value_portfolio_with_options` uses a fixed metric set from `standard_portfolio_metrics()`. For optimization we may need additional metrics (e.g. duration, WAL, time to maturity).

- **Planned change** (inside `portfolio::valuation`):

  ```rust
  #[derive(Clone, Debug)]
  pub struct PortfolioValuationOptions {
      /// When `true`, any failure to compute requested risk metrics for a
      /// position causes the entire portfolio valuation to fail.
      pub strict_risk: bool,

      /// Optional list of additional metrics to request from instruments.
      /// These are merged with standard_portfolio_metrics() unless
      /// `replace_standard_metrics` is true.
      pub additional_metrics: Option<Vec<MetricId>>,

      /// When true, `additional_metrics` replaces standard metrics entirely
      /// rather than being merged.
      pub replace_standard_metrics: bool,
  }

  impl Default for PortfolioValuationOptions {
      fn default() -> Self {
          Self {
              strict_risk: false,
              additional_metrics: None,
              replace_standard_metrics: false,
          }
      }
  }
  ```

- In `value_portfolio_*`:
  - Replace `let metrics = standard_portfolio_metrics();` with:

    ```rust
    let metrics: Vec<MetricId> = match (&options.additional_metrics, options.replace_standard_metrics) {
        (Some(additional), true) => additional.clone(),
        (Some(additional), false) => {
            let mut merged = standard_portfolio_metrics();
            for m in additional {
                if !merged.contains(m) {
                    merged.push(m.clone());
                }
            }
            merged
        }
        (None, _) => standard_portfolio_metrics(),
    };
    ```

- The optimization module can then construct a `PortfolioValuationOptions` with the union of:
  - `standard_portfolio_metrics()` (for DV01/CS01/etc., if desired).
  - Any additional metrics referenced by its `PerPositionMetric` definitions.

#### 5.2 Metric discovery and validation

- Add a helper in `optimization.rs`:

  ```rust
  /// Collect all MetricIds required by the problem's PerPositionMetrics.
  fn required_metrics(problem: &PortfolioOptimizationProblem) -> Vec<MetricId> {
      let mut metrics = Vec::new();

      // Helper to extract from PerPositionMetric
      let mut add_metric = |ppm: &PerPositionMetric| {
          if let PerPositionMetric::Metric(id) = ppm {
              if !metrics.contains(id) {
                  metrics.push(id.clone());
              }
          }
      };

      // Extract from objective
      match &problem.objective {
          Objective::Maximize(expr) | Objective::Minimize(expr) => {
              if let MetricExpr::WeightedSum { metric }
                   | MetricExpr::ValueWeightedAverage { metric } = expr {
                  add_metric(metric);
              }
          }
      }

      // Extract from constraints
      for constraint in &problem.constraints {
          if let Constraint::MetricBound { metric, .. } = constraint {
              if let MetricExpr::WeightedSum { metric: ppm }
                   | MetricExpr::ValueWeightedAverage { metric: ppm } = metric {
                  add_metric(ppm);
              }
          }
      }

      metrics
  }
  ```

- Before running the optimizer:
  - Compute `required_metrics(problem)`.
  - Build `PortfolioValuationOptions { strict_risk: false, additional_metrics: Some(required_metrics), .. }`.
  - Call `value_portfolio_with_options` to obtain a `PortfolioValuation` with all needed `ValuationResult::measures`.

- Error handling based on `missing_metric_policy`:
  - `Zero`: Use 0.0 for missing metrics.
  - `Exclude`: Remove position from decision universe.
  - `Strict`: Return error with details about which positions are missing which metrics.

---

### 6. Optimization Pipeline (Algorithm)

This section describes the step-by-step flow implemented by `DefaultLpOptimizer::optimize`.

#### 6.1 Step 0 – Validate problem

- Check that:
  - The portfolio has been `validate()`-d (or call `validate()` internally).
  - `weighting` is supported (for now: `ValueWeight` and `UnitScaling`).
  - `Budget` constraint exists or will be implicitly added (for `ValueWeight`, ensure `∑ w_i == 1`).
  - All constraints reference only supported `MetricExpr` variants.
  - At least one position is in the decision universe.
- On failure, return `PortfolioError::invalid_input(...)`.

#### 6.2 Step 1 – Build decision variable list from trade universe

Build the list of decision variables (tradeable items) from the trade universe:

- **Tradeable existing positions**:
  - Traverse `portfolio.positions` (in their current order).
  - Include positions where `trade_universe.is_tradeable(position)` is true.
  - These have `current_weight > 0` (computed from current PV).

- **Candidate positions**:
  - Append candidates from `trade_universe.candidates` (in order added).
  - These have `current_weight = 0` (not yet in portfolio).
  - Apply per-candidate `min_weight` / `max_weight` as variable bounds.

- **Held positions**:
  - Positions matching `trade_universe.held_filter` are NOT decision variables.
  - Their weight is fixed at current value and contributes to portfolio metrics.
  - The budget constraint accounts for held weight: `Σ w_tradeable = 1 - Σ w_held`.

- Build deterministic mappings:
  - `decision_items: Vec<DecisionItem>` – ordered list of decision variables.
  - `position_id_to_index: IndexMap<PositionId, usize>` – inverse mapping.

```rust
/// A decision variable in the optimization problem.
enum DecisionItem {
    /// Existing portfolio position that can be traded.
    ExistingPosition {
        position_id: PositionId,
        current_weight: f64,
        current_quantity: f64,
    },
    /// Candidate instrument not currently in portfolio.
    Candidate {
        candidate: CandidatePosition,
        // current_weight = 0, current_quantity = 0
    },
}
```

#### 6.3 Step 2 – Value portfolio and candidates with required metrics

- Compute `required_metrics = required_metrics(problem)`.
- Construct `PortfolioValuationOptions` with appropriate settings.

**Value existing portfolio**:
- Call:

  ```rust
  let valuation = value_portfolio_with_options(
      &problem.portfolio,
      market,
      config,
      &options,
  )?;
  ```

- From `valuation.position_values[position_id].valuation_result`, extract:
  - `measures: IndexMap<String, f64>`.
  - `value_base: Money` (for PV-based weights).

**Value candidate instruments**:
- For each candidate in `trade_universe.candidates`:
  - Call `candidate.instrument.price_with_metrics(market, as_of, &required_metrics)`.
  - Extract measures for constraint evaluation.
  - Note: candidates have `current_pv = 0` for weight calculation (they're not held yet).

- Build a **per-decision-item feature record**:

  ```rust
  struct DecisionFeatures {
      pv_base: f64,           // Current PV (0 for candidates)
      pv_per_unit: f64,       // PV per unit of the instrument
      measures: IndexMap<String, f64>,
      tags: IndexMap<String, String>,
      current_quantity: f64,  // 0 for candidates
      min_weight: f64,        // From candidate or 0 for existing
      max_weight: f64,        // From candidate or 1.0 for existing
  }
  ```

- Store in `IndexMap<PositionId, DecisionFeatures>` for deterministic iteration.

#### 6.4 Step 3 – Compute current weights

- For each decision position:
  - If `WeightingScheme::ValueWeight`:
    - Let `pv_i = features[i].pv_base`.
    - Let `total_pv = ∑ pv_i` over decision universe positions.
    - Current weight \( w_i^{(0)} = pv_i / total_pv \) (handle zero totals carefully).
  - If `WeightingScheme::UnitScaling`:
    - Interpret `w_i` as **multiplier of current quantity**, where current `w_i^{(0)} = 1.0`.
- Store current weights in the result for comparison.

#### 6.5 Step 4 – Lower MetricExpr to linear coefficients

- For each decision index `i`:
  - Compute `m_i(metric: &PerPositionMetric)`:
    - `Metric(id)` → `features[i].measures.get(id.as_str()).copied()` + missing metric policy.
    - `CustomKey(k)` → lookup by key string.
    - `PvBase` / `PvNative` → `features[i].pv_base` / `pv_native`.
    - `TagEquals { key, value }` → indicator 0.0 or 1.0.
    - `Constant(c)` → `c`.
- For each `MetricExpr`:
  - Build a coefficient vector `a: Vec<f64>` with `a[i] = m_i(...)`.
  - For:
    - `WeightedSum` → `a[i] = m_i`.
    - `ValueWeightedAverage` → `a[i] = m_i` (normalization via `Budget`).
    - `TagExposureShare` → `a[i] = 1.0` if tag matches, else `0.0`.

These `a` vectors are used for both the objective and constraints.

#### 6.6 Step 5 – Assemble LP model using good_lp

```rust
use good_lp::{constraint, default_solver, variable, Expression, Solution, SolverModel};

fn build_lp_model(
    n_vars: usize,
    objective_coeffs: &[f64],
    maximize: bool,
    constraints: &[LpConstraint],
    var_bounds: &[(f64, f64)],
) -> Result<impl SolverModel, PortfolioError> {
    // Create variables with bounds
    let mut vars = good_lp::ProblemVariables::new();
    let w: Vec<_> = (0..n_vars)
        .map(|i| {
            vars.add(variable()
                .min(var_bounds[i].0)
                .max(var_bounds[i].1))
        })
        .collect();

    // Build objective
    let obj: Expression = w.iter()
        .zip(objective_coeffs)
        .map(|(&v, &c)| c * v)
        .sum();

    let mut problem = if maximize {
        vars.maximise(obj)
    } else {
        vars.minimise(obj)
    };

    // Add constraints
    for c in constraints {
        let lhs: Expression = w.iter()
            .zip(&c.coefficients)
            .map(|(&v, &coef)| coef * v)
            .sum();

        match c.relation {
            LpRelation::Le => problem = problem.with(constraint!(lhs <= c.rhs)),
            LpRelation::Ge => problem = problem.with(constraint!(lhs >= c.rhs)),
            LpRelation::Eq => problem = problem.with(constraint!(lhs == c.rhs)),
        }
    }

    problem.using(default_solver)
}
```

- Let the decision vector be `w ∈ R^n`.
- Objective:
  - `Objective::Maximize(expr)` → maximize `a·w` for that `MetricExpr`.
  - `Objective::Minimize(expr)` → minimize `a·w`.
- Constraints:
  - `MetricBound { metric, op, rhs }`:
    - Build `a` for `metric`.
    - Add row based on `op`.
  - `TagExposureLimit`:
    - Equivalent to `MetricBound` with `TagExposureShare` and `Le max_share`.
  - `TagExposureMinimum`:
    - Equivalent to `MetricBound` with `TagExposureShare` and `Ge min_share`.
  - `WeightBounds { filter, min, max }`:
    - For each position matching `filter`, set variable bounds.
  - `MaxTurnover`:
    - Requires auxiliary variables for absolute values (see below).
  - `Budget { rhs }`:
    - Add row: `∑_i w_i = rhs`.
- Variable bounds:
  - MVP: `w_i ≥ 0` by default (long-only).
  - Later: allow negative weights (shorting) via configuration.

**Turnover constraints linearization**:

For `MaxTurnover`, we need `∑ |w_i - w_i^{(0)}| ≤ T`. This requires auxiliary variables:

```rust
// For each position i, introduce auxiliary variable t_i >= 0
// with constraints: t_i >= w_i - w_i^{(0)} and t_i >= w_i^{(0)} - w_i
// Then: ∑ t_i <= max_turnover
```

All matrices/vectors should be constructed in a **deterministic order** following `index_to_position_id`.

#### 6.7 Step 6 – Call LP solver

- Pass the assembled model to good_lp's solver.
- Configure for deterministic behavior.
- On solver return:
  - If **optimal / feasible**:
    - Extract solution `w*`.
    - Compute objective value `a·w*`.
    - Map weights back to `PositionId`s using `index_to_position_id`.
    - Extract dual values if available.
  - If **infeasible / unbounded**:
    - Set appropriate `OptimizationStatus` with diagnostics.

#### 6.8 Step 7 – Derive implied quantities and metric values

- Implied quantities:
  - `WeightingScheme::ValueWeight`:
    - Let `target_pv_i = w_i* * total_pv_universe`.
    - Convert `target_pv_i` to quantity using instrument's pricing / scaling. For MVP we can approximate:
      - `new_quantity_i = (w_i* / w_i^{(0)}) * current_quantity_i`, with guard for `w_i^{(0)} == 0`.
  - `WeightingScheme::UnitScaling`:
    - `new_quantity_i = w_i* * current_quantity_i`.
- Weight deltas:
  - `delta_i = w_i* - w_i^{(0)}`.
- Metric evaluation:
  - For any `MetricExpr` we want to display in results (e.g. portfolio duration, CCC weight), evaluate it at `w*`:
    - Reuse the `a` vector: `metric_value = a·w*`.
  - Store as `metric_values: IndexMap<String, f64>` with human-readable keys:
    - e.g. `"portfolio_duration_mod"`, `"ccc_weight"`, `"objective_ytm"`.

---

### 7. Example: Max Yield with Duration and CCC Constraints

Rewriting the user's example in terms of the new API:

> "Optimize the portfolio weights such that we have the highest yield subject to duration < 4 years and no more than 10% of the weight in CCC."

Assumptions:

- Each bond-like position exposes:
  - `MetricId::Ytm` – yield to maturity.
  - `MetricId::DurationMod` – modified duration.
- Each position carries a `rating` tag such as `"AAA"`, `"BBB"`, `"CCC"`.

```rust
use finstack_portfolio::optimization::*;
use finstack_valuations::metrics::MetricId;

// Basic optimization: trade all existing positions
let problem = PortfolioOptimizationProblem::new(
    my_portfolio.clone(),
    Objective::Maximize(MetricExpr::ValueWeightedAverage {
        metric: PerPositionMetric::Metric(MetricId::Ytm),
    }),
)
.with_constraints([
    // Duration <= 4.0 years
    Constraint::MetricBound {
        label: Some("duration_limit".into()),
        metric: MetricExpr::ValueWeightedAverage {
            metric: PerPositionMetric::Metric(MetricId::DurationMod),
        },
        op: Inequality::Le,
        rhs: 4.0,
    },
    // CCC weight <= 10%
    Constraint::TagExposureLimit {
        label: Some("ccc_limit".into()),
        tag_key: "rating".into(),
        tag_value: "CCC".into(),
        max_share: 0.10,
    },
    // Max turnover 25%
    Constraint::MaxTurnover {
        label: Some("turnover_limit".into()),
        max_turnover: 0.25,
    },
]);

let optimizer = DefaultLpOptimizer::default();
let result = optimizer.optimize(&problem, &market, &config)?;

println!("Optimization status: {:?}", result.status);
println!("Optimal objective (portfolio yield): {:.4}", result.objective_value);
println!("Optimal CCC weight: {:.2}%",
    result.metric_values.get("ccc_weight").unwrap_or(&0.0) * 100.0);
println!("Optimal duration: {:.2} years",
    result.metric_values.get("portfolio_duration_mod").unwrap_or(&0.0));
println!("Total turnover: {:.2}%", result.turnover() * 100.0);

// Generate trade list
let trades = result.to_trade_list();
for trade in &trades[..5.min(trades.len())] {
    println!("{}: {:?} {:.0} units ({:+.2}% weight)",
        trade.instrument_id, trade.direction,
        trade.delta_quantity, (trade.target_weight - trade.current_weight) * 100.0);
}

// Check binding constraints
let binding = result.binding_constraints();
println!("Binding constraints: {:?}", binding);
```

#### Example 2: Optimization with Hedging Candidates

Adding potential hedging instruments to the trade universe:

```rust
use finstack_portfolio::optimization::*;
use finstack_portfolio::position::PositionUnit;
use finstack_portfolio::types::DUMMY_ENTITY_ID;
use finstack_valuations::metrics::MetricId;

// Create candidate hedging positions
let hedge_candidates = vec![
    CandidatePosition::new(
        "HEDGE_CDX_IG",
        DUMMY_ENTITY_ID,
        Arc::new(cdx_ig_cds),
        PositionUnit::Notional(Some(Currency::USD)),
    )
    .with_tag("type", "hedge")
    .with_tag("asset_class", "credit")
    .with_max_weight(0.10),  // Max 10% in this hedge

    CandidatePosition::new(
        "HEDGE_RATE_FUTURE",
        DUMMY_ENTITY_ID,
        Arc::new(treasury_future),
        PositionUnit::Units,
    )
    .with_tag("type", "hedge")
    .with_tag("asset_class", "rates")
    .with_max_weight(0.15),
];

// Create trade universe: existing positions + hedging candidates
let universe = TradeUniverse::all_positions()
    .with_candidates(hedge_candidates)
    .with_held_positions(PositionFilter::ByTag {
        key: "locked".into(),
        value: "true".into(),
    });

let problem = PortfolioOptimizationProblem::new(
    my_portfolio.clone(),
    Objective::Maximize(MetricExpr::ValueWeightedAverage {
        metric: PerPositionMetric::Metric(MetricId::Ytm),
    }),
)
.with_trade_universe(universe)
.with_constraints([
    Constraint::MetricBound {
        label: Some("duration_limit".into()),
        metric: MetricExpr::ValueWeightedAverage {
            metric: PerPositionMetric::Metric(MetricId::DurationMod),
        },
        op: Inequality::Le,
        rhs: 4.0,
    },
    // Limit total hedge allocation
    Constraint::TagExposureLimit {
        label: Some("hedge_limit".into()),
        tag_key: "type".into(),
        tag_value: "hedge".into(),
        max_share: 0.20,  // Max 20% in hedges
    },
]);

let result = optimizer.optimize(&problem, &market, &config)?;

// Check which hedges were added
for (pos_id, &weight) in &result.optimal_weights {
    if weight > 0.001 && pos_id.as_str().starts_with("HEDGE_") {
        println!("Added hedge: {} at {:.2}% weight", pos_id, weight * 100.0);
    }
}
```

This example will be used as a golden test once the optimizer is implemented, with a small synthetic portfolio where yields, durations, and ratings are known analytically.

---

### 8. Error Handling

Add a new error variant for optimization-specific failures:

```rust
// In portfolio::error
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum PortfolioError {
    // ... existing variants ...

    /// Optimization error
    #[error("Optimization error: {0}")]
    OptimizationError(String),
}

impl PortfolioError {
    /// Create an optimization error
    pub fn optimization_error(msg: impl Into<String>) -> Self {
        Self::OptimizationError(msg.into())
    }
}
```

---

### 9. Determinism, Currency Safety, and Error Handling

- **Determinism**:
  - Use `IndexMap` for all maps to preserve insertion order.
  - Iterate portfolio positions in a consistent order when building decision vectors and coefficient matrices.
  - Configure LP solver for deterministic behavior; avoid parallel reductions inside the solver that could introduce non-deterministic floating-point order.
  - Use `good_lp` with single-threaded backend (minilp) for guaranteed determinism.
- **Currency safety**:
  - All value-based weights and PV-based metrics use **`value_base`** from `PortfolioValuation`, which is already converted to `portfolio.base_ccy`.
  - No implicit cross-currency arithmetic is introduced in the optimization layer.
- **Result envelope**:
  - `PortfolioOptimizationResult` includes `ResultsMeta` for policy stamping, consistent with `PortfolioResults` and `ValuationResult`.
- **Error handling**:
  - Reuse `PortfolioError` with new `OptimizationError` variant.
  - Ensure early, clear errors when:
    - Required metrics are missing (if `Strict` policy).
    - Universe selection yields zero decision positions.
    - Constraints are structurally unsatisfiable (simple checks where obvious).
    - Solver fails or times out.

---

### 10. Python Binding Sketch (Phase 2)

For later integration into `finstack-py`, we can mirror the Rust types with serde-friendly specs:

- Add `OptimizationProblemSpec` / `OptimizationResultSpec` in `finstack-py`:
  - Use enums/variants closely matching `WeightingScheme`, `MetricExpr`, `Objective`, `Constraint`.
  - Express `PerPositionMetric` in terms of:
    - Known metric IDs (string names matching `MetricId::as_str()`).
    - Tag operations.
    - PV-based metrics.
- Expose a Python function:

  ```python
  from finstack.portfolio import optimize_portfolio

  result = optimize_portfolio(problem_spec, market, config)

  # Access results
  print(f"Status: {result.status}")
  print(f"Optimal weights: {result.optimal_weights}")
  print(f"Trade list: {result.to_trade_list()}")
  ```

- The Python binding will:
  - Convert `OptimizationProblemSpec` → `PortfolioOptimizationProblem`.
  - Call `DefaultLpOptimizer::optimize`.
  - Convert `PortfolioOptimizationResult` → `OptimizationResultSpec`.

This phase should be implemented only after the Rust APIs have stabilized and at least one end-to-end Rust example (+ tests) is in place.

---

### 11. Implementation Checklist

#### Phase 1: MVP Core (Essential)

- [ ] **Step 1**: Add `optimization.rs` with the core types:
  - `WeightingScheme`, `PositionFilter`.
  - `TradeUniverse`, `CandidatePosition`.
  - `PerPositionMetric`, `MetricExpr`, `Objective`, `Inequality`, `Constraint`.
  - `MissingMetricPolicy`.
  - `PortfolioOptimizationProblem`, `OptimizationStatus`, `PortfolioOptimizationResult`.
  - `TradeSpec`, `TradeDirection`.
  - `PortfolioOptimizer` trait and `DefaultLpOptimizer`.

- [ ] **Step 2**: Add `PortfolioError::OptimizationError` variant.

- [ ] **Step 3**: Extend `PortfolioValuationOptions` with `additional_metrics` and `replace_standard_metrics`.

- [ ] **Step 4**: Implement helper functions in `optimization.rs`:
  - `required_metrics(problem)`.
  - Universe selection and indexing helpers.
  - Feature extraction from `PortfolioValuation` into `PositionFeatures`.
  - Lowering `PerPositionMetric` and `MetricExpr` into coefficient vectors.

- [ ] **Step 5**: Add `good_lp` dependency with feature flag.

- [ ] **Step 6**: Implement LP model assembly in `DefaultLpOptimizer`.

- [ ] **Step 7**: Implement `to_rebalanced_portfolio()` and `to_trade_list()` helpers.

- [ ] **Step 8**: Add Rust integration test:
  - Construct a toy bond/deposit portfolio with synthetic yields/durations and ratings.
  - Run the "max yield, duration < 4, CCC ≤ 10%" problem.
  - Assert the optimal allocation matches expectations.

#### Phase 2: Enhanced Features

- [ ] **Step 9**: Implement turnover constraints with auxiliary variables.
- [ ] **Step 10**: Add dual values / shadow prices extraction.
- [ ] **Step 11**: Improve infeasibility diagnostics.
- [ ] **Step 12**: Add constraint slack tracking.
- [ ] **Step 13**: Add benchmarks for performance validation.

#### Phase 3: Bindings

- [ ] **Step 14**: Add `optimization_spec.rs` with serde types.
- [ ] **Step 15**: Python bindings with Pydantic models.
- [ ] **Step 16**: WASM bindings with JSON IO.
- [ ] **Step 17**: Integration tests for bindings.

---

### 12. WASM/JS Serialization and Binding Plan

To ensure this optimization feature can be driven dynamically from a JS frontend via the WASM bindings, we separate **runtime types** (internal, non-serializable) from **spec types** (serde-only, wire format).

#### 12.1 Spec vs runtime types

- **Runtime (internal, non-serde or partially serde)**:
  - `PortfolioOptimizationProblem`
  - `PortfolioOptimizationResult`
  - `PortfolioOptimizer`, `DefaultLpOptimizer`
  - `Portfolio`, `MarketContext`, `FinstackConfig`
  - `MetricId`
- **Wire/spec (serde, JS/WASM-safe)**:
  - `OptimizationProblemSpec`
  - `OptimizationResultSpec`
  - `WeightingSchemeSpec`, `PositionFilterSpec`, `DecisionUniverseSpec`
  - `PerPositionMetricSpec`, `MetricExprSpec`, `ObjectiveSpec`, `ConstraintSpec`, `InequalitySpec`

Spec types use only:

- Primitive scalars (`bool`, `f64`, `String`),
- Collections (`Vec`, `IndexMap<String, T>`),
- Enums with serde-friendly `tag`/`content` representations.

No trait objects, `Arc<dyn ...>`, or non-serializable handles appear in specs.

#### 12.2 OptimizationProblemSpec shape

```rust
/// Wire format for constructing an optimization problem from JS/WASM.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationProblemSpec {
    pub weighting: WeightingSchemeSpec,

    /// Trade universe: which positions to trade and candidates to consider.
    /// Default: all existing positions tradeable, no candidates.
    #[serde(default)]
    pub trade_universe: TradeUniverseSpec,

    pub objective: ObjectiveSpec,
    pub constraints: Vec<ConstraintSpec>,

    #[serde(default)]
    pub missing_metric_policy: MissingMetricPolicySpec,

    /// How to identify the portfolio to optimize.
    /// Option A: Embed a full PortfolioSpec (for stateless calls).
    pub portfolio_spec: Option<PortfolioSpec>,

    /// Option B: Reference a previously-registered portfolio by ID
    /// in the WASM runtime.
    pub portfolio_id: Option<String>,

    pub label: Option<String>,
    #[serde(default)]
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}
```

Key points:

- The spec does **not** contain `Portfolio` directly, only `PortfolioSpec` or an ID.
- Exactly one of `portfolio_spec` or `portfolio_id` must be present; this is enforced in conversion code.
- Metric references use `String` IDs on the wire (e.g., `"duration_mod"`, `"ytm"`) and are mapped to `MetricId` internally.

#### 12.3 Spec enums (examples)

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WeightingSchemeSpec {
    ValueWeight,
    NotionalWeight,
    UnitScaling,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PositionFilterSpec {
    All,
    ByEntityId { entity_id: String },
    ByTag { key: String, value: String },
    ByPositionIds { position_ids: Vec<String> },
    Not { filter: Box<PositionFilterSpec> },
}

/// Candidate position specification for WASM/JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePositionSpec {
    pub id: String,
    pub entity_id: String,
    /// Instrument specification (JSON representation).
    pub instrument_spec: InstrumentJson,
    /// Unit type: "units", "face_value", "percentage", or {"notional": "USD"}.
    pub unit: serde_json::Value,
    #[serde(default)]
    pub tags: indexmap::IndexMap<String, String>,
    #[serde(default = "default_max_weight")]
    pub max_weight: f64,
    #[serde(default)]
    pub min_weight: f64,
}

fn default_max_weight() -> f64 { 1.0 }

/// Trade universe specification for WASM/JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeUniverseSpec {
    /// Filter for tradeable existing positions. Default: all.
    #[serde(default = "default_all_filter")]
    pub tradeable_filter: PositionFilterSpec,

    /// Filter for positions to hold constant. Default: none.
    #[serde(default)]
    pub held_filter: Option<PositionFilterSpec>,

    /// Candidate instruments to consider adding.
    #[serde(default)]
    pub candidates: Vec<CandidatePositionSpec>,

    /// Allow shorting candidates. Default: false.
    #[serde(default)]
    pub allow_short_candidates: bool,
}

fn default_all_filter() -> PositionFilterSpec { PositionFilterSpec::All }

impl Default for TradeUniverseSpec {
    fn default() -> Self {
        Self {
            tradeable_filter: PositionFilterSpec::All,
            held_filter: None,
            candidates: Vec::new(),
            allow_short_candidates: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PerPositionMetricSpec {
    /// Standard metric by ID string, e.g. "duration_mod", "ytm", "dv01".
    Metric { id: String },
    /// Custom metric key.
    CustomKey { key: String },
    PvBase,
    PvNative,
    TagEquals { key: String, value: String },
    Constant { value: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MetricExprSpec {
    WeightedSum { metric: PerPositionMetricSpec },
    ValueWeightedAverage { metric: PerPositionMetricSpec },
    TagExposureShare { tag_key: String, tag_value: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ObjectiveSpec {
    Maximize { expr: MetricExprSpec },
    Minimize { expr: MetricExprSpec },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InequalitySpec {
    Le,
    Ge,
    Eq,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingMetricPolicySpec {
    Zero,
    Exclude,
    Strict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ConstraintSpec {
    MetricBound {
        label: Option<String>,
        metric: MetricExprSpec,
        op: InequalitySpec,
        rhs: f64,
    },
    TagExposureLimit {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        max_share: f64,
    },
    TagExposureMinimum {
        label: Option<String>,
        tag_key: String,
        tag_value: String,
        min_share: f64,
    },
    WeightBounds {
        label: Option<String>,
        filter: PositionFilterSpec,
        min: f64,
        max: f64,
    },
    MaxTurnover {
        label: Option<String>,
        max_turnover: f64,
    },
    MaxPositionDelta {
        label: Option<String>,
        filter: PositionFilterSpec,
        max_delta: f64,
    },
    Budget { rhs: f64 },
}
```

These are intentionally isomorphic to the runtime enums but with:

- `MetricId` → `String`
- No references to `Portfolio`, `MarketContext`, or `FinstackConfig`.

#### 12.4 OptimizationResultSpec shape

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationResultSpec {
    pub status: OptimizationStatusSpec,

    /// Pre-trade weights: { position_id: weight }.
    pub current_weights: indexmap::IndexMap<String, f64>,

    /// Post-trade weights: { position_id: weight }.
    pub optimal_weights: indexmap::IndexMap<String, f64>,

    /// Weight changes: { position_id: delta }.
    pub weight_deltas: indexmap::IndexMap<String, f64>,

    /// Implied target quantities for each position.
    pub implied_quantities: indexmap::IndexMap<String, f64>,

    /// Objective value at optimum.
    pub objective_value: f64,

    /// Selected portfolio metrics evaluated at optimum.
    pub metric_values: indexmap::IndexMap<String, f64>,

    /// Dual values / shadow prices for constraints.
    pub dual_values: indexmap::IndexMap<String, f64>,

    /// Constraint slack values.
    pub constraint_slacks: indexmap::IndexMap<String, f64>,

    /// Total turnover.
    pub turnover: f64,

    /// Trade list.
    pub trades: Vec<TradeSpecJson>,

    /// Optional label/meta echoed from the problem.
    pub label: Option<String>,
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeSpecJson {
    pub position_id: String,
    pub instrument_id: String,
    pub trade_type: String,  // "existing", "newPosition", "closeOut"
    pub current_quantity: f64,
    pub target_quantity: f64,
    pub delta_quantity: f64,
    pub direction: String,   // "buy", "sell", "hold"
    pub current_weight: f64,
    pub target_weight: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OptimizationStatusSpec {
    Optimal,
    FeasibleButSuboptimal,
    Infeasible { conflicting_constraints: Vec<String> },
    Unbounded,
    Error { message: String },
}
```

This type is fully serializable and has no runtime-only fields; it is safe to return directly to JS/WASM as JSON.

#### 12.5 Conversion between spec and runtime

- Implement conversions on the Rust side:

  ```rust
  impl TryFrom<OptimizationProblemSpec> for PortfolioOptimizationProblem {
      type Error = PortfolioError;

      fn try_from(spec: OptimizationProblemSpec) -> Result<Self, Self::Error> {
          // 1. Resolve portfolio: from spec.portfolio_spec or spec.portfolio_id
          // 2. Map WeightingSchemeSpec -> WeightingScheme
          // 3. Map DecisionUniverseSpec -> DecisionUniverse
          // 4. Map PerPositionMetricSpec -> PerPositionMetric
          //    - String metric IDs -> MetricId via FromStr
          // 5. Map MetricExprSpec / ObjectiveSpec / ConstraintSpec similarly.
      }
  }

  impl From<PortfolioOptimizationResult> for OptimizationResultSpec {
      fn from(result: PortfolioOptimizationResult) -> Self {
          // 1. Copy status, objective_value, label, meta
          // 2. Map PositionId -> String keys in optimal_weights / implied_quantities
          // 3. Copy metric_values as-is (String keys)
          // 4. Generate trade list
          // 5. Calculate turnover
      }
  }
  ```

- All conversions are **pure Rust** and do not require any JS/WASM awareness.
- Errors (e.g. unknown metric name, missing portfolio) are mapped to `PortfolioError::invalid_input` or `PortfolioError::OptimizationError`.

#### 12.6 WASM entrypoint sketch

At the WASM boundary we expose a JSON-in/JSON-out function that only uses the spec types:

```rust
#[wasm_bindgen]
pub fn optimize_portfolio_js(
    problem_spec_json: &str,
    market_json: &str,
    config_json: &str,
) -> Result<String, JsValue> {
    // 1. Deserialize OptimizationProblemSpec from JSON
    let spec: OptimizationProblemSpec = serde_json::from_str(problem_spec_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid problem spec: {}", e)))?;

    // 2. Deserialize MarketContext and FinstackConfig
    let market: MarketContext = serde_json::from_str(market_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid market: {}", e)))?;
    let config: FinstackConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

    // 3. Convert to runtime problem
    let problem: PortfolioOptimizationProblem = spec
        .try_into()
        .map_err(|e: PortfolioError| JsValue::from_str(&e.to_string()))?;

    // 4. Run optimizer
    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer
        .optimize(&problem, &market, &config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 5. Convert to spec and serialize to JSON
    let result_spec: OptimizationResultSpec = result.into();
    let json = serde_json::to_string(&result_spec)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(json)
}
```

By constraining the WASM boundary to **spec types only**, we guarantee that:

- Every optimization problem constructed from JS is fully describable by JSON.
- Every result returned to JS is likewise JSON-serializable.
- Runtime-only objects (`Portfolio`, `MarketContext`, `FinstackConfig`, solver state) never cross the boundary directly; they are managed inside the Rust/WASM runtime.

---

### 13. Future Roadmap (Post-MVP)

Features correctly scoped out but should be on the roadmap:

1. **Risk parity weighting**: `RiskParity` as a special objective type
2. **Factor exposure constraints**: Limit exposure to specific risk factors
3. **Hierarchical constraints**: Entity-level weight limits (sum of positions per entity)
4. **Cash management**: Designate positions as "liquid/adjustable" vs "hold"
5. **Multi-period optimization**: Rebalancing paths over time
6. **Transaction cost modeling**: Include trading costs in objective
7. **QP support**: Variance minimization, tracking error
8. **Scenario-aware optimization**: Robust/worst-case constraints
