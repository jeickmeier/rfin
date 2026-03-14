//! Commonly used types and functions.
//!
//! Import this module to get quick access to the most common types:
//!
//! ```rust
//! use finstack_portfolio::prelude::*;
//! ```

pub use crate::attribution::{attribute_portfolio_pnl, PortfolioAttribution};
pub use crate::book::{Book, BookId};
pub use crate::builder::PortfolioBuilder;
pub use crate::cashflows::{aggregate_cashflows, PortfolioCashflows};
pub use crate::dependencies::{DependencyIndex, MarketFactorKey};
pub use crate::error::{Error, Result};
pub use crate::grouping::{aggregate_by_attribute, aggregate_by_book, group_by_attribute};
pub use crate::margin::{
    NettingSet, NettingSetManager, NettingSetMargin, PortfolioMarginAggregator,
    PortfolioMarginResult,
};
pub use crate::metrics::{aggregate_metrics, AggregatedMetric, PortfolioMetrics};
pub use crate::optimization::{
    optimize_max_yield_with_ccc_limit, CandidatePosition, Constraint, DefaultLpOptimizer,
    Inequality, MaxYieldWithCccLimitResult, MetricExpr, MissingMetricPolicy, Objective,
    PerPositionMetric, PortfolioOptimizationProblem, PortfolioOptimizationResult, PositionFilter,
    TradeDirection, TradeSpec, TradeType, TradeUniverse, WeightingScheme,
};
pub use crate::portfolio::{Portfolio, PortfolioSpec};
pub use crate::position::{Position, PositionUnit};
pub use crate::results::PortfolioResult;
pub use crate::types::{Entity, EntityId, PositionId, DUMMY_ENTITY_ID};
pub use crate::valuation::{revalue_affected, value_portfolio, PortfolioValuation, PositionValue};

#[cfg(feature = "scenarios")]
pub use crate::scenarios::{apply_and_revalue, apply_scenario};

// Re-export the full core prelude for a unified foundation
pub use finstack_core::prelude::*;
