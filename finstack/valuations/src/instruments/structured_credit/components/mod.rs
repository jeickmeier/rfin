//! Components for structured credit instruments.
//!
//! This module contains calculation and engine components for structured credit.
//! Type definitions have been moved to the `types` module for cleaner organization.
//!
//! # Module Organization
//!
//! ## Calculation Components
//!
//! - [`waterfall`]: Tier-based payment distribution engine
//! - [`coverage_tests`]: OC/IC test calculations for waterfall diversion
//! - [`diversion`]: Diversion rules with circular reference detection
//! - [`validation`]: Validation framework for waterfall specifications
//! - [`rates`]: Rate conversion utilities (CPR↔SMM, CDR↔MDR, PSA→CPR)
//! - [`rate_helpers`]: Floating rate projection helpers
//! - [`tranche_valuation`]: Tranche-level metrics (WAL, duration, Z-spread, CS01)
//!
//! ## Behavioral Models
//!
//! - [`market_context`]: Market conditions and credit factors for behavioral models
//!
//! ## Stochastic Components
//!
//! - [`stochastic`]: Complete stochastic framework including:
//!   - Correlation structures (single-factor, multi-factor)
//!   - Default models (copula-based, intensity process)
//!   - Prepayment models (factor-correlated, Richard-Roll)
//!   - Scenario tree infrastructure
//!   - Stochastic pricer and metrics
//!
//! # Type Definitions
//!
//! Type definitions (enums, pool, tranches, setup) are now in `crate::instruments::structured_credit::types`.

// ============================================================================
// CALCULATION COMPONENTS
// ============================================================================

pub mod coverage_tests;
pub mod diversion;
pub mod rate_helpers;
pub mod rates;
pub mod tranche_valuation;
pub mod validation;
pub mod waterfall;

// ============================================================================
// BEHAVIORAL MODELS
// ============================================================================

pub mod market_context;

// ============================================================================
// STOCHASTIC COMPONENTS
// ============================================================================

pub mod stochastic;

// ============================================================================
// RE-EXPORTS FROM TYPES MODULE (for backward compatibility)
// ============================================================================

// Core enums and classifications - re-exported from types
pub use super::types::{AssetType, DealType, PaymentMode, TrancheSeniority, TriggerConsequence};

// Pool structure - re-exported from types
pub use super::types::{
    calculate_pool_stats, AssetPool, ConcentrationCheckResult, ConcentrationViolation, PoolAsset,
    PoolStats, ReinvestmentCriteria, ReinvestmentPeriod,
};

// Tranche structure - re-exported from types
pub use super::types::{
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

// Tranche-level types (metrics moved to metrics/ module)
pub use tranche_valuation::{TrancheCashflowResult, TrancheValuation, TrancheValuationExt};

// Tranche-level metrics (re-exported from metrics/ for backward compatibility)
pub use super::metrics::{
    calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
    calculate_tranche_z_spread,
};

// ============================================================================
// DETERMINISTIC RE-EXPORTS (single-path behavioral models)
// ============================================================================

// Market context for behavioral models
pub use market_context::{CreditFactors, MarketConditions, MarketFactors};

// Deterministic behavioral curves (PSA, SDA, constant CPR/CDR)
// Re-exported from cashflow builder as single source of truth
pub use crate::cashflow::builder::{
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
