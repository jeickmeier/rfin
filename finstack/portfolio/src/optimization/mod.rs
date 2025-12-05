#![allow(clippy::module_name_repetitions)]

//! Portfolio optimization on top of valuations.
//!
//! This module provides a deterministic, metric‑driven portfolio optimization
//! facility that operates entirely on top of existing valuation results.

mod constraints;
mod decision;
mod lp_solver;
mod problem;
mod result;
mod types;
mod universe;

pub use constraints::{Constraint, Inequality};
pub use lp_solver::{DefaultLpOptimizer, PortfolioOptimizer};
pub use problem::PortfolioOptimizationProblem;
pub use result::{
    OptimizationStatus, PortfolioOptimizationResult, TradeDirection, TradeSpec, TradeType,
};
pub use types::{MetricExpr, MissingMetricPolicy, Objective, PerPositionMetric, WeightingScheme};
pub use universe::{CandidatePosition, PositionFilter, TradeUniverse};
