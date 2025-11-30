//! Pricing and cashflow projection for structured credit instruments.
//!
//! This module contains pure functions for:
//! - Deterministic cashflow simulation
//! - Waterfall execution
//! - Coverage test evaluation
//! - Diversion rule processing
//! - Stochastic pricing

pub mod coverage_tests;
pub mod deterministic;
pub mod diversion;
pub mod stochastic;
pub mod waterfall;

// Re-export deterministic functions
pub use coverage_tests::{CoverageTest, TestContext, TestResult};
pub use deterministic::{generate_cashflows, generate_tranche_cashflows, run_simulation};
pub use diversion::{DiversionCondition, DiversionEngine, DiversionRule};
pub use waterfall::{execute_waterfall, execute_waterfall_with_workspace};

// Re-export stochastic types
pub use stochastic::{
    // Scenario tree infrastructure
    BranchingSpec,
    // Default models
    CopulaBasedDefault,
    // Risk metrics and sensitivities
    CorrelationSensitivities,
    // Correlation structures
    CorrelationStructure,
    // Prepayment models
    FactorCorrelatedPrepay,
    IntensityProcessDefault,
    // Stochastic pricing engine
    PricingMode,
    RichardRollPrepay,
    ScenarioNode,
    ScenarioNodeId,
    ScenarioPath,
    ScenarioTree,
    ScenarioTreeConfig,
    SensitivityConfig,
    StochasticDefault,
    StochasticDefaultSpec,
    StochasticMetrics,
    StochasticMetricsCalculator,
    StochasticPrepaySpec,
    StochasticPrepayment,
    StochasticPricer,
    StochasticPricerConfig,
    StochasticPricingResult,
    TranchePricingResult,
};
