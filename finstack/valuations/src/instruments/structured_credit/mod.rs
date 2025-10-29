//! Unified structured credit instrument module.
//!
//! This module consolidates the previously separate ABS, CLO, CMBS, and RMBS
//! implementations into a single `StructuredCredit` type, eliminating ~1,400 lines
//! of near-duplicate code.
//!
//! # Module Organization
//!
//! - `components/`: All structural and behavioral building blocks
//!   - Structural: enums, pool, tranches, waterfall, coverage_tests
//!   - Behavioral: specs (prepayment/default/recovery), market_context
//!   - Valuation: tranche_valuation, rates
//! - `metrics/`: Risk and valuation metrics organized by category
//! - `types.rs`: Main StructuredCredit instrument type
//! - `pricer.rs`: Pricing and valuation logic
//! - `config.rs`: Constants and deal configuration
//! - `utils.rs`: Date utilities, rating factors, reinvestment
//! - `instrument_trait.rs`: Internal trait for cashflow generation

pub mod components;
pub mod config;
pub mod instrument_trait;
pub mod metrics;
pub mod pricer;
pub mod types;
pub mod utils;

/// Prelude module for end-users.
///
/// Import this module to get the most commonly used types for working with
/// structured credit instruments.
///
/// See unit tests and `examples/` for usage.
pub mod prelude {
    // Main types
    pub use super::DealType;
    pub use super::StructuredCredit;

    // Building blocks
    pub use super::calculate_pool_stats;
    pub use super::AssetPool;
    pub use super::PoolAsset;
    pub use super::Tranche;
    pub use super::TrancheBuilder;
    pub use super::TrancheCoupon;
    pub use super::TrancheSeniority;
    pub use super::TrancheStructure;

    // Behavioral models (as specs for serialization)
    pub use super::DefaultModelSpec;
    pub use super::PrepaymentModelSpec;
    pub use super::RecoveryModelSpec;

    // Metadata and overrides
    pub use super::BehaviorOverrides;
    pub use super::DealMetadata;

    // Enums
    pub use super::AssetType;
    pub use super::CreditRating;

    // Results
    pub use super::TrancheCashflowResult;
    pub use super::TrancheValuation;
    pub use super::TrancheValuationExt;

    // Configuration
    pub use super::DealConfig;
    pub use super::DealDates;
    pub use super::DealFees;
}

// ============================================================================
// Main instrument type and pricer
// ============================================================================

pub use pricer::StructuredCreditDiscountingPricer;
pub use types::{BehaviorOverrides, DealMetadata, StructuredCredit};

// ============================================================================
// Components (structural + behavioral)
// ============================================================================

pub use components::{
    calculate_pool_stats,
    calculate_tranche_cs01,
    calculate_tranche_duration,
    calculate_tranche_wal,
    calculate_tranche_z_spread,
    cdr_to_mdr,
    // Rate conversion utilities
    cpr_to_smm,
    mdr_to_cdr,
    psa_to_cpr,
    smm_to_cpr,
    // Pool
    AssetPool,
    // Enumerations
    AssetType,
    ConcentrationCheckResult,
    ConcentrationViolation,
    // Coverage tests
    CoverageTest,
    CoverageTestType,
    CoverageTrigger,
    CreditEnhancement,
    CreditFactors,
    CreditRating,
    DealType,
    DefaultModelSpec,
    ManagementFeeType,
    // Market context
    MarketConditions,
    MarketFactors,
    PaymentCalculation,
    PaymentMode,
    PaymentRecipient,
    PaymentRecord,
    PaymentRule,
    PoolAsset,
    PoolStats,
    // Behavioral model specs (single source of truth)
    PrepaymentModelSpec,
    RecoveryModelSpec,
    ReinvestmentCriteria,
    ReinvestmentPeriod,
    TestContext,
    TestResult,
    // Tranches
    Tranche,
    TrancheBehaviorType,
    TrancheBuilder,
    // Tranche valuation
    TrancheCashflowResult,
    TrancheCoupon,
    TrancheSeniority,
    TrancheStructure,
    TrancheValuation,
    TrancheValuationExt,
    TriggerConsequence,
    WaterfallBuilder,
    // Waterfall
    WaterfallEngine,
    WaterfallResult,
};

pub use utils::months_between;

// ============================================================================
// Metrics (re-exported for convenience)
// ============================================================================

pub use metrics::{
    // Registration function
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
// Configuration and utilities
// ============================================================================

pub use config::{
    CoverageTestConfig,
    // Deal configuration
    DealConfig,
    DealDates,
    DealFees,
    DefaultAssumptions,
    ABS_SERVICING_FEE_BPS,
    BASELINE_UNEMPLOYMENT_RATE,
    BASIS_POINTS_DIVISOR,
    CLO_SENIOR_MGMT_FEE_BPS,
    CLO_SUBORDINATED_MGMT_FEE_BPS,
    CLO_TRUSTEE_FEE_ANNUAL,
    CMBS_MASTER_SERVICER_FEE_BPS,
    CMBS_SPECIAL_SERVICER_FEE_BPS,
    CREDIT_CARD_SEASONALITY,
    // Constants
    DAYS_PER_YEAR,
    DEFAULT_AUTO_ABS_SPEED,
    DEFAULT_AUTO_RAMP_MONTHS,
    DEFAULT_BURNOUT_THRESHOLD_MONTHS,
    DEFAULT_MAX_COV_LITE,
    DEFAULT_MAX_DIP,
    DEFAULT_MAX_OBLIGOR_CONCENTRATION,
    DEFAULT_MAX_SECOND_LIEN,
    DEFAULT_MAX_TOP10_CONCENTRATION,
    DEFAULT_MAX_TOP5_CONCENTRATION,
    DEFAULT_RESOLUTION_LAG_MONTHS,
    MIN_PREPAYMENT_RATE,
    MONTHS_PER_YEAR,
    MORTGAGE_SEASONALITY,
    PERCENTAGE_MULTIPLIER,
    POOL_BALANCE_CLEANUP_THRESHOLD,
    PSA_RAMP_MONTHS,
    PSA_TERMINAL_CPR,
    QUARTERLY_PERIODS_PER_YEAR,
    RMBS_SERVICING_FEE_BPS,
    SDA_PEAK_CDR,
    SDA_PEAK_MONTH,
    SDA_TERMINAL_CDR,
    STANDARD_CDR_RATES,
    STANDARD_PSA_SPEEDS,
    STANDARD_SEVERITY_RATES,
};

pub use utils::{
    moodys_warf_factor,
    // Rating factors
    RatingFactorTable,
    // Reinvestment
    ReinvestmentManager,
};

// ============================================================================
// Type aliases for backward compatibility
// ============================================================================

pub type StructuredCreditWaterfall = WaterfallEngine;
