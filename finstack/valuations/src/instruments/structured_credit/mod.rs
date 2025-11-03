//! Structured credit instruments: ABS, RMBS, CMBS, and CLO with waterfall modeling.
//!
//! Provides comprehensive modeling for asset-backed securities with:
//! - Collateral pool management (prepayment, default, recovery)
//! - Multi-tranche capital structure with seniority
//! - Sequential-pay and pro-rata waterfall logic
//! - Overcollateralization and coverage tests
//! - Deal-specific metrics (WAL, WARF, WAS, DSCR, LTV)
//!
//! This unified implementation consolidates ABS, RMBS, CMBS, and CLO into a single
//! `StructuredCredit` type with deal-type-specific behaviors, eliminating ~1,400 lines
//! of duplicate code while maintaining full feature parity.
//!
//! # Structured Credit Overview
//!
//! Structured credit securities are backed by pools of loans or receivables,
//! with cashflows distributed to multiple tranches based on seniority and
//! performance triggers. The fundamental structure:
//!
//! **Assets → Pool Cashflows → Waterfall → Tranches**
//!
//! # Deal Types
//!
//! ## ABS (Asset-Backed Securities)
//! - **Collateral**: Auto loans, credit cards, student loans, equipment leases
//! - **Prepayment**: CPR or ABS speed curve
//! - **Default**: CDR with severity
//! - **Key metrics**: Charge-off rate, delinquency, excess spread
//!
//! ## RMBS (Residential Mortgage-Backed Securities)
//! - **Collateral**: Residential mortgages
//! - **Prepayment**: PSA model (Public Securities Association curve)
//! - **Default**: SDA model (Standard Default Assumption)
//! - **Key metrics**: FICO score, LTV, WAC, WAM
//!
//! ## CMBS (Commercial Mortgage-Backed Securities)
//! - **Collateral**: Commercial real estate mortgages
//! - **Prepayment**: Lockout, defeasance, yield maintenance
//! - **Default**: Property-level modeling
//! - **Key metrics**: DSCR (debt service coverage), LTV, property type concentration
//!
//! ## CLO (Collateralized Loan Obligations)
//! - **Collateral**: Leveraged loans (typically BB/B rated)
//! - **Prepayment**: Loan repayment and refinancing
//! - **Default**: Obligor default with recovery
//! - **Key metrics**: WARF (weighted average rating factor), WAS (weighted average spread)
//!
//! # Waterfall Mechanics
//!
//! Cashflows from the collateral pool are distributed via waterfall:
//!
//! 1. **Fees**: Servicing, trustee, management fees
//! 2. **Senior interest**: Most senior tranche gets interest
//! 3. **Subordinate interest**: Lower tranches get interest (if coverage tests pass)
//! 4. **Principal**: Sequential-pay (senior first) or pro-rata
//! 5. **Equity**: Residual to equity tranche
//!
//! ## Sequential Pay
//! Principal pays down tranches in order of seniority until fully redeemed.
//!
//! ## Pro Rata
//! Principal distributed proportionally across tranches.
//!
//! ## Coverage Tests
//! If tests fail, principal may be redirected to senior tranches (turbo):
//! - **OC test**: Overcollateralization ratio
//! - **IC test**: Interest coverage ratio
//!
//! # Pricing Methodology
//!
//! Structured credit pricing requires:
//!
//! 1. **Pool cashflow projection**: Apply prepayment/default/recovery models
//! 2. **Waterfall execution**: Distribute pool cashflows per deal rules
//! 3. **Tranche valuation**: Discount tranche cashflows at appropriate spread
//!
//! ```text
//! Tranche_PV = Σ Tranche_CF(t) × DF(t, spread)
//! ```
//!
//! # Behavioral Models
//!
//! ## Prepayment Models
//! - **PSA (RMBS)**: Public Securities Association curve (100% PSA = 0.2% → 6% CPR over 30 months)
//! - **CPR (General)**: Constant prepayment rate
//! - **ABS speed**: Deal-specific speed curves
//!
//! ## Default Models
//! - **CDR**: Constant default rate
//! - **SDA (RMBS)**: Standard Default Assumption curve
//! - **Vintage curves**: Historical cohort performance
//!
//! ## Recovery Models
//! - **Constant severity**: Fixed loss given default (e.g., 40%)
//! - **Time-varying**: Recovery varies with market conditions
//! - **Collateral-based**: Recovery from underlying asset value
//!
//! # Key Metrics by Deal Type
//!
//! **ABS Metrics:**
//! - Charge-off rate, Delinquency rate, Excess spread, Speed
//!
//! **RMBS Metrics:**
//! - Weighted average FICO, Weighted average LTV, WAC, WAM, CPR, CDR
//!
//! **CMBS Metrics:**
//! - DSCR, LTV, Property type concentration, WAC
//!
//! **CLO Metrics:**
//! - WARF (rating factor), WAS (spread), Obligor concentration, Covenant compliance
//!
//! **Common Metrics:**
//! - WAL (weighted average life), Duration, Convexity, Z-spread, OAS
//! - DV01, CS01, Prepayment01, Default01, Recovery01, Severity01
//!
//! # References
//!
//! ## Industry Standards
//!
//! - S&P Global Ratings. "CDO Evaluator Applies Correlation and Monte Carlo
//!   Simulation to Determine Portfolio Quality." Ratings Direct, various editions.
//!
//! - Moody's Investors Service. "WARF®: Measuring Portfolio Credit Risk in CLOs."
//!   Special Comment, December 2009.
//!
//! ## Academic References
//!
//! - Hu, J. (2007). "Assessing the Credit Risk of CDOs Backed by Structured Finance
//!   Securities: Rating Analysts' Challenges and Solutions." *Working Paper*, Federal
//!   Reserve Bank of Chicago.
//!
//! - Duffie, D., & Gârleanu, N. (2001). "Risk and Valuation of Collateralized Debt
//!   Obligations." *Financial Analysts Journal*, 57(1), 41-59.
//!
//! - Goodman, L. S., Li, S., Lucas, D. J., Zimmerman, T. A., & Fabozzi, F. J. (2008).
//!   *Subprime Mortgage Credit Derivatives*. Wiley.
//!
//! ## Prepayment and Default Models
//!
//! - PSA Standard: Public Securities Association (1985). "PSA Prepayment Model."
//!
//! - Schwartz, E. S., & Torous, W. N. (1989). "Prepayment and the Valuation of
//!   Mortgage-Backed Securities." *Journal of Finance*, 44(2), 375-392.
//!
//! # Implementation Notes
//!
//! - **Unified type**: Single `StructuredCredit` handles all deal types
//! - **Deal-specific behavior**: Controlled via `DealType` enum
//! - **Waterfall engine**: Generic waterfall with configurable rules
//! - **Performance**: Optimized for repeated scenario analysis
//! - **Serialization**: Full serde support for deal specifications
//!
//! # Module Organization
//!
//! - [`components`]: Structural elements (pool, tranches, waterfall, coverage tests)
//! - [`metrics`]: Risk metrics organized by category (pricing, risk, pool, deal-specific)
//! - [`types`]: Main `StructuredCredit` instrument type
//! - [`pricer`]: Waterfall execution and valuation
//! - [`config`]: Deal configuration, fees, and industry constants
//! - [`utils`]: Rating factors, reinvestment logic, date utilities
//!
//! # See Also
//!
//! - [`StructuredCredit`] for main instrument struct
//! - [`DealType`] for ABS/RMBS/CMBS/CLO specification
//! - [`AssetPool`] for collateral pool modeling
//! - [`Tranche`] for tranche structure
//! - [`WaterfallEngine`] for cashflow distribution
//! - [`PrepaymentModelSpec`] for prepayment behavior
//! - [`DefaultModelSpec`] for default behavior
//! - [`RecoveryModelSpec`] for recovery modeling

pub mod components;
pub mod config;
pub mod instrument_trait;
pub mod metrics;
pub mod pricer;
pub mod templates;
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
    // Diversion system
    DiversionCondition,
    DiversionEngine,
    DiversionRule,
    ManagementFeeType,
    // Market context
    MarketConditions,
    MarketFactors,
    PaymentCalculation,
    PaymentMode,
    PaymentRecipient,
    PaymentRecord,
    PaymentType,
    PoolAsset,
    PoolStats,
    // Behavioral model specs (single source of truth)
    PrepaymentModelSpec,
    Recipient,
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
    // Validation
    ValidationError,
    WaterfallBuilder,
    // Waterfall
    WaterfallEngine,
    WaterfallResult,
    WaterfallTier,
    WaterfallValidator,
    AllocationMode,
    get_validation_errors,
    is_valid_waterfall_spec,
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
// Waterfall Templates
// ============================================================================

pub use templates::{
    available_templates, clo_2_0_template, cmbs_standard_template, cre_operating_company_template,
    get_template, WaterfallTemplate,
};

