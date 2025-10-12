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
//!
//! ## Behavioral Models
//! - `specs`: Behavioral model specifications (prepayment, default, recovery)
//! - `market_context`: Market conditions and credit factors
//! - `rates`: Rate conversion utilities (CPR/SMM, CDR/MDR, PSA)
//!
//! ## Valuation
//! - `tranche_valuation`: Tranche-specific cashflow generation and metrics

// Structural components
pub mod enums;
pub mod pool;
pub mod tranches;
pub mod waterfall;
pub mod coverage_tests;

// Behavioral models
pub mod specs;
pub mod market_context;

// Valuation
pub mod tranche_valuation;

// Utilities
pub mod rates;

// ============================================================================
// Re-export structural components
// ============================================================================

pub use enums::{
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
};

pub use pool::{
    AssetPool, PoolAsset, PoolStats, ReinvestmentPeriod, ReinvestmentCriteria,
    ConcentrationCheckResult, ConcentrationViolation, calculate_pool_stats,
};

pub use tranches::{
    Tranche, TrancheBuilder, TrancheStructure, TrancheCoupon,
    TrancheBehaviorType, CoverageTrigger, CreditEnhancement,
};

pub use waterfall::{
    WaterfallEngine, WaterfallBuilder, WaterfallResult, PaymentRule,
    PaymentRecipient, PaymentCalculation, ManagementFeeType,
    CoverageTrigger as WaterfallCoverageTrigger, CoverageTestType,
    PaymentRecord,
};

pub use coverage_tests::{
    CoverageTest, TestContext, TestResult,
};

// ============================================================================
// Re-export behavioral models
// ============================================================================

// Market context structures
pub use market_context::{
    MarketConditions, CreditFactors, MarketFactors,
};

// Behavioral model specifications (single source of truth)
pub use specs::{
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
};

// Rate conversion utilities
pub use rates::{cpr_to_smm, smm_to_cpr, cdr_to_mdr, mdr_to_cdr, psa_to_cpr};

// ============================================================================
// Re-export tranche valuation
// ============================================================================

pub use tranche_valuation::{
    TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
    calculate_tranche_wal, calculate_tranche_duration,
    calculate_tranche_z_spread, calculate_tranche_cs01,
};
