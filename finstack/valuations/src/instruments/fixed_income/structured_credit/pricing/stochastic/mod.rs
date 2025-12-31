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
//! - [`calibrations`]: Standard calibration constants for RMBS, CLO, CMBS
//! - [`prepayment`]: Stochastic prepayment models (factor-correlated, Richard-Roll)
//! - [`default`]: Stochastic default models (copula-based, intensity process)
//! - [`correlation`]: Correlation structures for structured credit
//! - [`tree`]: Scenario tree infrastructure for path-dependent analysis
//! - [`pricer`]: Stochastic pricing engine with tree and MC modes
//! - [`metrics`]: Risk metrics and correlation sensitivities

pub mod calibrations;
pub mod correlation;
pub mod default;
pub(crate) mod metrics;
pub mod prepayment;
pub(crate) mod pricer;
pub mod tree;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests;

// Re-export main types (may be used by external bindings or tests)
#[allow(unused_imports)]
pub use calibrations::{
    CloCalibration, CmbsCalibration, RmbsCalibration, CLO_STANDARD, CMBS_STANDARD, RMBS_STANDARD,
};
pub use correlation::CorrelationStructure;
#[allow(unused_imports)]
pub use default::{
    CopulaBasedDefault, IntensityProcessDefault, StochasticDefault, StochasticDefaultSpec,
};
#[allow(unused_imports)]
pub use metrics::{
    CorrelationSensitivities, SensitivityConfig, StochasticMetrics, StochasticMetricsCalculator,
};
#[allow(unused_imports)]
pub use prepayment::{
    FactorCorrelatedPrepay, RichardRollPrepay, StochasticPrepaySpec, StochasticPrepayment,
};
#[allow(unused_imports)] // May be used by external bindings
pub use pricer::{
    PricingMode, StochasticPricer, StochasticPricerConfig, StochasticPricingResult,
    TranchePricingResult,
};
#[allow(unused_imports)]
pub use tree::{
    BranchingSpec, ScenarioNode, ScenarioNodeId, ScenarioPath, ScenarioTree, ScenarioTreeConfig,
};
