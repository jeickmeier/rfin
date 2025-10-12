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
pub mod metrics;
pub mod pricer;
pub mod types;
pub mod config;
pub mod utils;
pub mod instrument_trait;

/// Prelude module for end-users.
///
/// Import this module to get the most commonly used types for working with
/// structured credit instruments:
///
/// ```rust,ignore
/// use finstack_valuations::instruments::structured_credit::prelude::*;
///
/// let clo = StructuredCredit::builder()
///     .deal_type(DealType::CLO)
///     .pool(pool)
///     .tranches(tranches)
///     .legal_maturity(maturity)
///     .disc_id("USD-OIS".into())
///     .build()?;
/// ```
pub mod prelude {
    // Main types
    pub use super::StructuredCredit;
    pub use super::DealType;
    
    // Building blocks
    pub use super::AssetPool;
    pub use super::PoolAsset;
    pub use super::Tranche;
    pub use super::TrancheBuilder;
    pub use super::TrancheSeniority;
    pub use super::TrancheStructure;
    pub use super::TrancheCoupon;
    pub use super::calculate_pool_stats;
    
    // Behavioral models (as specs for serialization)
    pub use super::PrepaymentModelSpec;
    pub use super::DefaultModelSpec;
    pub use super::RecoveryModelSpec;
    
    // Metadata and overrides
    pub use super::DealMetadata;
    pub use super::BehaviorOverrides;
    
    // Enums
    pub use super::CreditRating;
    pub use super::AssetType;
    
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
    // Enumerations
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
    // Pool
    AssetPool, PoolAsset, PoolStats, ReinvestmentPeriod, ReinvestmentCriteria,
    ConcentrationCheckResult, ConcentrationViolation, calculate_pool_stats,
    // Tranches
    Tranche, TrancheBuilder, TrancheStructure, TrancheCoupon,
    TrancheBehaviorType, CoverageTrigger, CreditEnhancement,
    // Waterfall
    WaterfallEngine, WaterfallBuilder, WaterfallResult, PaymentRule,
    PaymentRecipient, PaymentCalculation, ManagementFeeType,
    CoverageTestType, PaymentRecord,
    // Coverage tests
    CoverageTest, TestContext, TestResult,
    // Market context
    MarketConditions, CreditFactors, MarketFactors,
    // Behavioral model specs (single source of truth)
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
    // Rate conversion utilities
    cpr_to_smm, smm_to_cpr, cdr_to_mdr, mdr_to_cdr, psa_to_cpr,
    // Tranche valuation
    TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
    calculate_tranche_wal, calculate_tranche_duration,
    calculate_tranche_z_spread, calculate_tranche_cs01,
};

pub use utils::months_between;

// ============================================================================
// Metrics (re-exported for convenience)
// ============================================================================

pub use metrics::{
    // Pricing metrics
    AccruedCalculator, CleanPriceCalculator, DirtyPriceCalculator, WalCalculator,
    // Risk metrics
    MacaulayDurationCalculator, ModifiedDurationCalculator, YtmCalculator,
    ZSpreadCalculator, Cs01Calculator, SpreadDurationCalculator,
    // Pool metrics
    WamCalculator, CprCalculator, CdrCalculator, CloWarfCalculator, CloWasCalculator,
    // Deal-specific metrics
    AbsChargeOffCalculator, AbsCreditEnhancementCalculator,
    AbsDelinquencyCalculator, AbsExcessSpreadCalculator, AbsSpeedCalculator,
    CloWalCalculator, CmbsDscrCalculator, CmbsLtvCalculator,
    RmbsFicoCalculator, RmbsLtvCalculator, RmbsWalCalculator,
    // Registration function
    register_structured_credit_metrics,
};

// ============================================================================
// Configuration and utilities
// ============================================================================

pub use config::{
    // Constants
    DAYS_PER_YEAR, QUARTERLY_PERIODS_PER_YEAR,
    BASIS_POINTS_DIVISOR, PERCENTAGE_MULTIPLIER, MONTHS_PER_YEAR,
    MORTGAGE_SEASONALITY, CREDIT_CARD_SEASONALITY,
    BASELINE_UNEMPLOYMENT_RATE, MIN_PREPAYMENT_RATE,
    STANDARD_PSA_SPEEDS, STANDARD_CDR_RATES, STANDARD_SEVERITY_RATES,
    CLO_SENIOR_MGMT_FEE_BPS, CLO_SUBORDINATED_MGMT_FEE_BPS,
    ABS_SERVICING_FEE_BPS, CMBS_MASTER_SERVICER_FEE_BPS,
    CMBS_SPECIAL_SERVICER_FEE_BPS, RMBS_SERVICING_FEE_BPS,
    CLO_TRUSTEE_FEE_ANNUAL, POOL_BALANCE_CLEANUP_THRESHOLD,
    DEFAULT_RESOLUTION_LAG_MONTHS, PSA_RAMP_MONTHS, PSA_TERMINAL_CPR,
    DEFAULT_AUTO_ABS_SPEED, DEFAULT_AUTO_RAMP_MONTHS,
    SDA_PEAK_MONTH, SDA_PEAK_CDR, SDA_TERMINAL_CDR,
    DEFAULT_BURNOUT_THRESHOLD_MONTHS,
    DEFAULT_MAX_OBLIGOR_CONCENTRATION, DEFAULT_MAX_TOP5_CONCENTRATION,
    DEFAULT_MAX_TOP10_CONCENTRATION, DEFAULT_MAX_SECOND_LIEN,
    DEFAULT_MAX_COV_LITE, DEFAULT_MAX_DIP,
    // Deal configuration
    DealConfig, DealDates, DealFees, CoverageTestConfig, DefaultAssumptions,
};

pub use utils::{
    // Rating factors
    RatingFactorTable, moodys_warf_factor,
    // Reinvestment
    ReinvestmentManager,
};

// ============================================================================
// Type aliases for backward compatibility
// ============================================================================

pub type StructuredCreditWaterfall = WaterfallEngine;
