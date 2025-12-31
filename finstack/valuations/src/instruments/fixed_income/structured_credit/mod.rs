//! Structured credit instruments: ABS, RMBS, CMBS, and CLO with waterfall modeling.
//!
//! Provides comprehensive modeling for asset-backed securities with:
//! - Collateral pool management (prepayment, default, recovery)
//! - Multi-tranche capital structure with seniority
//! - Sequential-pay and pro-rata waterfall logic
//! - Overcollateralization and coverage tests
//! - Deal-specific metrics (WAL, WARF, WAS, DSCR, LTV)
//!
//! # Module Organization
//!
//! - [`types`]: All data structures (instrument, pool, tranches, waterfall, results)
//! - [`pricing`]: Pure functions for cashflow simulation and waterfall execution
//! - [`metrics`]: Risk metrics organized by category
//! - [`utils`]: Helper functions (rate conversions, validation)
//!
//! # See Also
//!
//! - [`StructuredCredit`] for main instrument struct
//! - [`DealType`] for ABS/RMBS/CMBS/CLO specification
//! - [`Pool`] for collateral pool modeling
//! - [`Tranche`] for tranche structure
//! - [`Waterfall`] for cashflow distribution

// ============================================================================
// MODULES
// ============================================================================

pub mod config {
    //! Configuration and constants for structured credit instruments.

    /// Industry-standard constants for structured credit modeling.
    pub mod constants {
        pub use crate::instruments::structured_credit::types::constants::*;
    }
    pub use crate::instruments::structured_credit::types::setup::{
        CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions,
    };
    pub use constants::*;
}

// New module structure
pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod pricing;
pub(crate) mod types;
pub(crate) mod utils;

// ============================================================================
// PRELUDE
// ============================================================================

/// Prelude module for end-users.
pub mod prelude {
    // Main types
    pub use super::DealType;
    pub use super::StructuredCredit;

    // Building blocks
    pub use super::Pool;
    pub use super::PoolAsset;
    pub use super::Tranche;
    pub use super::TrancheBuilder;
    pub use super::TrancheCoupon;
    pub use super::TrancheStructure;

    // Seniority
    pub use super::Seniority;

    // Waterfall
    pub use super::Waterfall;
    pub use super::WaterfallBuilder;
    pub use super::WaterfallTier;

    // Behavioral models
    pub use super::DefaultModelSpec;
    pub use super::PrepaymentModelSpec;
    pub use super::RecoveryModelSpec;

    // Metadata and overrides
    pub use super::Metadata;
    pub use super::Overrides;

    // Enums
    pub use super::AssetType;

    // Results
    pub use super::TrancheCashflows;
    pub use super::TrancheValuation;
    pub use super::TrancheValuationExt;

    // Configuration
    pub use super::DealConfig;
    pub use super::DealDates;
    pub use super::DealFees;
}

// ============================================================================
// MAIN TYPES
// ============================================================================

pub use pricer::StructuredCreditDiscountingPricer;
pub use types::{
    // Pool types
    calculate_pool_stats,
    // Waterfall types
    AllocationMode,
    // Enums
    AssetType,
    // Metadata
    ConcentrationCheckResult,
    ConcentrationViolation,
    // Stochastic specs
    CorrelationStructure,
    // Configuration
    CoverageTestConfig,
    CoverageTestType,
    // Tranche types
    CoverageTrigger,
    CreditEnhancement,
    DealConfig,
    DealDates,
    DealFees,
    DealType,
    DefaultAssumptions,
    ManagementFeeType,
    Metadata,
    Overrides,
    PaymentCalculation,
    PaymentMode,
    PaymentRecord,
    PaymentType,
    Pool,
    PoolAsset,
    PoolStats,
    Recipient,
    RecipientType,
    ReinvestmentCriteria,
    // Reinvestment
    ReinvestmentManager,
    ReinvestmentPeriod,
    RepLine,
    RoundingConvention,
    Seniority,
    StochasticDefaultSpec,
    StochasticPrepaySpec,
    // Main instrument
    StructuredCredit,
    Tranche,
    TrancheBehaviorType,
    TrancheBuilder,
    // Result types
    TrancheCashflows,
    TrancheCoupon,
    TrancheStructure,
    TrancheValuation,
    TrancheValuationExt,
    TriggerConsequence,
    Waterfall,
    WaterfallBuilder,
    WaterfallDistribution,
    WaterfallTier,
    WaterfallWorkspace,
};

// Behavioral models
pub use crate::cashflow::builder::{DefaultCurve, PrepaymentCurve};
pub use types::{
    CreditFactors, DefaultModelSpec, MarketConditions, PrepaymentModelSpec, RecoveryModelSpec,
};

// ============================================================================
// UTILITIES
// ============================================================================

pub use utils::{
    cdr_to_mdr, cpr_to_smm, get_validation_errors, is_valid_waterfall_spec, mdr_to_cdr, psa_to_cpr,
    smm_to_cpr, ValidationError,
};

// ============================================================================
// PRICING FUNCTIONS
// ============================================================================

#[doc(hidden)]
pub use pricing::stochastic::PricingMode;
#[doc(hidden)]
pub use pricing::waterfall::execute_waterfall_with_explanation;
pub use pricing::{
    execute_waterfall, execute_waterfall_with_workspace, generate_cashflows,
    generate_tranche_cashflows, run_simulation,
};

pub use pricing::coverage_tests::{CoverageTest, TestContext, TestResult};
pub use pricing::diversion::{DiversionCondition, DiversionEngine, DiversionRule};
#[doc(hidden)]
pub use pricing::waterfall::WaterfallContext;

// ============================================================================
// METRICS
// ============================================================================

pub use metrics::{
    calculate_tranche_cs01,
    calculate_tranche_duration,
    calculate_tranche_wal,
    calculate_tranche_z_spread,
    register_structured_credit_metrics,
    // Deal-specific metrics
    AbsChargeOffCalculator,
    AbsCreditEnhancementCalculator,
    AbsDelinquencyCalculator,
    AbsExcessSpreadCalculator,
    AbsSpeedCalculator,
    // Pricing metrics
    AccruedCalculator,
    CdrCalculator,
    CleanPriceCalculator,
    CloWalCalculator,
    CloWarfCalculator,
    CloWasCalculator,
    CmbsDscrCalculator,
    CmbsLtvCalculator,
    CprCalculator,
    Cs01Calculator,
    DirtyPriceCalculator,
    // Risk metrics
    MacaulayDurationCalculator,
    ModifiedDurationCalculator,
    RmbsFicoCalculator,
    RmbsLtvCalculator,
    RmbsWalCalculator,
    SpreadDurationCalculator,
    WalCalculator,
    // Pool metrics
    WamCalculator,
    YtmCalculator,
    ZSpreadCalculator,
};

// ============================================================================
// CONSTANTS
// ============================================================================

pub use types::constants::{
    ABS_SERVICING_FEE_BPS, BASELINE_UNEMPLOYMENT_RATE, BASIS_POINTS_DIVISOR,
    CLO_SENIOR_MGMT_FEE_BPS, CLO_SUBORDINATED_MGMT_FEE_BPS, CLO_TRUSTEE_FEE_ANNUAL,
    CMBS_MASTER_SERVICER_FEE_BPS, CMBS_SPECIAL_SERVICER_FEE_BPS, CREDIT_CARD_SEASONALITY,
    DAYS_PER_YEAR, DEFAULT_AUTO_ABS_SPEED, DEFAULT_AUTO_RAMP_MONTHS,
    DEFAULT_BURNOUT_THRESHOLD_MONTHS, DEFAULT_MAX_COV_LITE, DEFAULT_MAX_DIP,
    DEFAULT_MAX_OBLIGOR_CONCENTRATION, DEFAULT_MAX_SECOND_LIEN, DEFAULT_MAX_TOP10_CONCENTRATION,
    DEFAULT_MAX_TOP5_CONCENTRATION, DEFAULT_RESOLUTION_LAG_MONTHS, MIN_PREPAYMENT_RATE,
    MONTHS_PER_YEAR, MORTGAGE_SEASONALITY, PERCENTAGE_MULTIPLIER, POOL_BALANCE_CLEANUP_THRESHOLD,
    PSA_RAMP_MONTHS, PSA_TERMINAL_CPR, QUARTERLY_PERIODS_PER_YEAR, RMBS_SERVICING_FEE_BPS,
    SDA_PEAK_CDR, SDA_PEAK_MONTH, SDA_TERMINAL_CDR, STANDARD_CDR_RATES, STANDARD_PSA_SPEEDS,
    STANDARD_SEVERITY_RATES,
};

// Re-export waterfall coverage trigger with clear name
/// Waterfall-level coverage trigger (for waterfall diversion).
/// Use this when building waterfall engines with coverage test diversion.
pub use types::waterfall::CoverageTrigger as WaterfallCoverageTrigger;

/// Re-exports for backward compatibility with WASM bindings.
#[doc(hidden)]
pub mod waterfall {
    pub use super::types::waterfall::{CoverageTestRules, CoverageTrigger};
}
