//! Components for structured credit instruments.
//!
//! This module contains all the building blocks for structured credit:
//!
//! ## Structural Components
//! - `enums`: Deal types, credit ratings, asset classifications
//! - `pool`: Asset pool structure and statistics
//! - `tranches`: Tranche structure with attachment/detachment points
//! - `waterfall`: Payment distribution engine
//! - `coverage_tests`: OC/IC test calculations for waterfall diversion
//! - `diversion`: Diversion rules with circular detection
//! - `validation`: Validation framework for waterfall specifications
//!
//! ## Behavioral Models
//! - `specs`: Behavioral model specifications (prepayment, default, recovery)
//! - `market_context`: Market conditions and credit factors
//! - `rates`: Rate conversion utilities (CPR/SMM, CDR/MDR, PSA)
//!
//! ## Valuation
//! - `tranche_valuation`: Tranche-specific cashflow generation and metrics

// Structural components
pub mod coverage_tests;
pub mod diversion;
pub mod enums;
pub mod pool;
pub mod tranches;
pub mod validation;
pub mod waterfall;

// Behavioral models
pub mod market_context;
pub mod specs;

// Valuation
pub mod tranche_valuation;

// Utilities
pub mod rate_helpers;
pub mod rates;

// Stochastic models
pub mod stochastic;

// ============================================================================
// Re-export structural components
// ============================================================================

pub use enums::{
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
};

pub use pool::{
    calculate_pool_stats, AssetPool, ConcentrationCheckResult, ConcentrationViolation, PoolAsset,
    PoolStats, ReinvestmentCriteria, ReinvestmentPeriod,
};

pub use tranches::{
    CoverageTrigger, CreditEnhancement, Tranche, TrancheBehaviorType, TrancheBuilder,
    TrancheCoupon, TrancheStructure,
};

pub use waterfall::{
    AllocationMode, CoverageTestType, CoverageTrigger as WaterfallCoverageTrigger,
    ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRecord, PaymentType, Recipient,
    WaterfallBuilder, WaterfallEngine, WaterfallResult, WaterfallTier, WaterfallWorkspace,
};

pub use coverage_tests::{CoverageTest, TestContext, TestResult};

// Diversion system
pub use diversion::{DiversionCondition, DiversionEngine, DiversionRule};

// Validation framework
pub use validation::{
    get_validation_errors, is_valid_waterfall_spec, ValidationError, WaterfallValidator,
};

// ============================================================================
// Re-export behavioral models
// ============================================================================

// Market context structures
pub use market_context::{CreditFactors, MarketConditions, MarketFactors};

// Behavioral model specifications (re-exported from builder)
pub use specs::{
    DefaultCurve, DefaultModelSpec, PrepaymentCurve, PrepaymentModelSpec, RecoveryModelSpec,
};

// Rate conversion utilities
pub use rates::{cdr_to_mdr, cpr_to_smm, mdr_to_cdr, psa_to_cpr, smm_to_cpr};

// Rate helpers for floating rate calculations
pub use rate_helpers::tenor_to_period_end;

// ============================================================================
// Re-export tranche valuation
// ============================================================================

pub use tranche_valuation::{
    calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
    calculate_tranche_z_spread, TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
};

// ============================================================================
// Re-export stochastic models
// ============================================================================

pub use stochastic::{
    // Scenario tree infrastructure
    BranchingSpec,
    // Default models
    CopulaBasedDefault,
    // Risk metrics and sensitivities
    CorrelationSensitivities,
    // Correlation
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
