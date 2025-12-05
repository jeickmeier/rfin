use super::constraints::Constraint;
use super::types::{MissingMetricPolicy, Objective, WeightingScheme};
use super::universe::TradeUniverse;
use crate::portfolio::Portfolio;
use indexmap::IndexMap;

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
