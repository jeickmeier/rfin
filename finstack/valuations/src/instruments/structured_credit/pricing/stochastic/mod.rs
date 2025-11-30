//! Stochastic models for structured credit.
//!
//! This module provides stochastic prepayment and default models with:
//! - Factor-driven CPR/CDR models with correlation
//! - Scenario tree infrastructure for path-dependent valuation
//! - Industry-standard calibrations (RMBS, CLO, CMBS)
//! - Stochastic pricing engine with NPV and risk metrics
//! - Comprehensive risk metrics and correlation sensitivities
//!
//! # Module Organization
//!
//! - [`prepayment`]: Stochastic prepayment models (factor-correlated, Richard-Roll)
//! - [`default`]: Stochastic default models (copula-based, intensity process)
//! - [`correlation`]: Correlation structures for structured credit
//! - [`tree`]: Scenario tree infrastructure for path-dependent analysis
//! - [`pricer`]: Stochastic pricing engine with tree and MC modes
//! - [`metrics`]: Risk metrics and correlation sensitivities

pub mod correlation;
pub mod default;
pub mod metrics;
pub mod prepayment;
pub mod pricer;
pub mod tree;

#[cfg(test)]
mod tests;

// Re-export main types
pub use correlation::CorrelationStructure;
pub use default::{
    CopulaBasedDefault, IntensityProcessDefault, StochasticDefault, StochasticDefaultSpec,
};
pub use metrics::{
    CorrelationSensitivities, SensitivityConfig, StochasticMetrics, StochasticMetricsCalculator,
};
pub use prepayment::{
    FactorCorrelatedPrepay, RichardRollPrepay, StochasticPrepaySpec, StochasticPrepayment,
};
pub use pricer::{
    PricingMode, StochasticPricer, StochasticPricerConfig, StochasticPricingResult,
    TranchePricingResult,
};
pub use tree::{
    BranchingSpec, ScenarioNode, ScenarioNodeId, ScenarioPath, ScenarioTree, ScenarioTreeConfig,
};
