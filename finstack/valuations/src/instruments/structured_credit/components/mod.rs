//! Components for structured credit instruments.
//!
//! This module contains all building blocks for structured credit, organized by
//! whether they support deterministic pricing, stochastic pricing, or both.
//!
//! # Module Organization
//!
//! ## Common Components (Both Deterministic & Stochastic)
//!
//! Core structural types and utilities used by all pricing modes:
//!
//! - [`enums`]: Deal types, asset classifications, payment modes
//! - [`pool`]: Asset pool structure and statistics
//! - [`tranches`]: Tranche structure with attachment/detachment points
//! - [`waterfall`]: Tier-based payment distribution engine
//! - [`coverage_tests`]: OC/IC test calculations for waterfall diversion
//! - [`diversion`]: Diversion rules with circular reference detection
//! - [`validation`]: Validation framework for waterfall specifications
//! - [`rates`]: Rate conversion utilities (CPR↔SMM, CDR↔MDR, PSA→CPR)
//! - [`rate_helpers`]: Floating rate projection helpers
//! - [`tranche_valuation`]: Tranche-level metrics (WAL, duration, Z-spread, CS01)
//!
//! ## Deterministic Components
//!
//! Single-path behavioral models for standard pricing:
//!
//! - [`specs`]: Deterministic behavioral curves (PSA, SDA, constant CPR/CDR)
//! - [`market_context`]: Market conditions and credit factors for behavioral models
//!
//! ## Stochastic Components
//!
//! Multi-path simulation models for advanced analytics:
//!
//! - [`stochastic`]: Complete stochastic framework including:
//!   - Correlation structures (single-factor, multi-factor)
//!   - Default models (copula-based, intensity process)
//!   - Prepayment models (factor-correlated, Richard-Roll)
//!   - Scenario tree infrastructure
//!   - Stochastic pricer and metrics
//!
//! # Pricing Mode Selection
//!
//! | Use Case | Components | When to Use |
//! |----------|------------|-------------|
//! | Standard pricing | Common + Deterministic | Day-to-day valuation, reporting |
//! | Risk analytics | Common + Stochastic | VaR, scenario analysis, correlation risk |
//! | Stress testing | Common + Either | Depends on scenario complexity |

// ============================================================================
// COMMON COMPONENTS (used by both deterministic and stochastic pricing)
// ============================================================================

pub mod coverage_tests;
pub mod diversion;
pub mod enums;
pub mod pool;
pub mod rate_helpers;
pub mod rates;
pub mod tranche_valuation;
pub mod tranches;
pub mod validation;
pub mod waterfall;

// ============================================================================
// DETERMINISTIC COMPONENTS (single-path behavioral models)
// ============================================================================

pub mod market_context;
pub mod specs;

// ============================================================================
// STOCHASTIC COMPONENTS (multi-path simulation models)
// ============================================================================

pub mod stochastic;

// ============================================================================
// COMMON RE-EXPORTS (used by both deterministic and stochastic pricing)
// ============================================================================

// Core enums and classifications
pub use enums::{AssetType, DealType, PaymentMode, TrancheSeniority, TriggerConsequence};

// Pool structure
pub use pool::{
    calculate_pool_stats, AssetPool, ConcentrationCheckResult, ConcentrationViolation, PoolAsset,
    PoolStats, ReinvestmentCriteria, ReinvestmentPeriod,
};

// Tranche structure
pub use tranches::{
    CoverageTrigger, CreditEnhancement, Tranche, TrancheBehaviorType, TrancheBuilder,
    TrancheCoupon, TrancheStructure,
};

// Waterfall engine
pub use waterfall::{
    AllocationMode, CoverageTestType, CoverageTrigger as WaterfallCoverageTrigger,
    ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRecord, PaymentType, Recipient,
    WaterfallBuilder, WaterfallEngine, WaterfallResult, WaterfallTier, WaterfallWorkspace,
};

// Coverage tests and diversion
pub use coverage_tests::{CoverageTest, TestContext, TestResult};
pub use diversion::{DiversionCondition, DiversionEngine, DiversionRule};

// Validation
pub use validation::{
    get_validation_errors, is_valid_waterfall_spec, ValidationError, WaterfallValidator,
};

// Rate utilities
pub use rate_helpers::tenor_to_period_end;
pub use rates::{cdr_to_mdr, cpr_to_smm, mdr_to_cdr, psa_to_cpr, smm_to_cpr};

// Tranche-level valuation metrics
pub use tranche_valuation::{
    calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
    calculate_tranche_z_spread, TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
};

// ============================================================================
// DETERMINISTIC RE-EXPORTS (single-path behavioral models)
// ============================================================================

// Market context for behavioral models
pub use market_context::{CreditFactors, MarketConditions, MarketFactors};

// Deterministic behavioral curves (PSA, SDA, constant CPR/CDR)
pub use specs::{
    DefaultCurve, DefaultModelSpec, PrepaymentCurve, PrepaymentModelSpec, RecoveryModelSpec,
};

// ============================================================================
// STOCHASTIC RE-EXPORTS (multi-path simulation models)
// ============================================================================

pub use stochastic::{
    // Correlation structures
    CorrelationStructure,
    // Default models
    CopulaBasedDefault,
    IntensityProcessDefault,
    StochasticDefault,
    StochasticDefaultSpec,
    // Prepayment models
    FactorCorrelatedPrepay,
    RichardRollPrepay,
    StochasticPrepaySpec,
    StochasticPrepayment,
    // Scenario tree infrastructure
    BranchingSpec,
    ScenarioNode,
    ScenarioNodeId,
    ScenarioPath,
    ScenarioTree,
    ScenarioTreeConfig,
    // Stochastic pricing engine
    PricingMode,
    StochasticPricer,
    StochasticPricerConfig,
    StochasticPricingResult,
    TranchePricingResult,
    // Risk metrics and sensitivities
    CorrelationSensitivities,
    SensitivityConfig,
    StochasticMetrics,
    StochasticMetricsCalculator,
};
