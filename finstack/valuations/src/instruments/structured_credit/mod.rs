//! Unified structured credit instrument module.
//!
//! This module consolidates the previously separate ABS, CLO, CMBS, and RMBS
//! implementations into a single `StructuredCredit` type, eliminating ~1,400 lines
//! of near-duplicate code.
//!
//! # Module Organization
//!
//! - `components/`: All structural and behavioral building blocks
//!   - Structural: pool, tranches, waterfall, enums
//!   - Behavioral: prepayment, default, recovery models
//!   - Valuation: tranche-specific cashflows and pricing
//! - `metrics/`: Risk and valuation metrics organized by category
//! - `types.rs`: Main StructuredCredit instrument type
//! - `pricer.rs`: Pricing and valuation logic
//! - `config.rs`: Constants and deal configuration
//! - `coverage_tests.rs`: Coverage tests (OC/IC)
//! - `utils.rs`: Rating factors and reinvestment utilities

pub mod components;
pub mod metrics;
pub mod pricer;
pub mod types;
pub mod config;
pub mod coverage_tests;
pub mod utils;
pub mod instrument_trait;

// ============================================================================
// Main instrument type and pricer
// ============================================================================

pub use pricer::StructuredCreditDiscountingPricer;
pub use types::{InstrumentSpecificFields, StructuredCredit};

// ============================================================================
// Components (structural + behavioral)
// ============================================================================

pub use components::{
    // Enumerations
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
    // Pool
    AssetPool, PoolAsset, PoolStats, ReinvestmentPeriod, ReinvestmentCriteria,
    ConcentrationCheckResult, ConcentrationViolation,
    // Tranches
    Tranche, TrancheBuilder, TrancheStructure, TrancheCoupon,
    TrancheBehaviorType, CoverageTrigger, CreditEnhancement,
    // Waterfall
    WaterfallEngine, WaterfallBuilder, WaterfallResult, PaymentRule,
    PaymentRecipient, PaymentCalculation, ManagementFeeType,
    CoverageTestType, PaymentRecord,
    // Prepayment models
    PrepaymentBehavior, PSAModel, CPRModel, VectorModel, AnnualStepCprModel,
    MarketConditions, calculate_seasoning_months, cpr_to_smm, smm_to_cpr,
    psa_to_cpr, prepayment_model_for, psa_model, cpr_model, vector_model,
    // Default and recovery models
    DefaultBehavior, RecoveryBehavior, CDRModel, SDAModel, VectorDefaultModel,
    MortgageDefaultModel, AutoDefaultModel, CreditCardChargeOffModel,
    ConstantRecoveryModel, CollateralRecoveryModel, AnnualStepCdrModel,
    CreditFactors, MarketFactors, cdr_to_mdr, mdr_to_cdr,
    default_model_for, recovery_model_for,
    // Serializable model specs
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
    // Tranche valuation
    TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
    calculate_tranche_wal, calculate_tranche_duration,
    calculate_tranche_z_spread, calculate_tranche_cs01,
};

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
    DAYS_PER_YEAR, VALIDATION_TOLERANCE, QUARTERLY_PERIODS_PER_YEAR,
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

pub use coverage_tests::{CoverageTest, TestContext, TestResult};

pub use utils::{
    // Rating factors
    RatingFactorTable, moodys_warf_factor,
    // Reinvestment
    ReinvestmentManager, EligibilityCriteria, ConcentrationLimits,
};

// ============================================================================
// Instrument trait
// ============================================================================

pub use instrument_trait::StructuredCreditInstrument;

// Type alias for backward compatibility
pub type StructuredCreditWaterfall = WaterfallEngine;
