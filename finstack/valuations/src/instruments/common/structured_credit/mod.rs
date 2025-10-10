//! Shared building blocks for structured credit instruments.
//!
//! This module provides a streamlined framework for modeling CLO, ABS, RMBS, CMBS,
//! and other structured credit products with:
//!
//! - Type-safe enumerations for deal types, asset types, and ratings
//! - Asset pool management with concentration limits and eligibility criteria  
//! - Tranche structures with attachment/detachment points
//! - Sequential waterfall engine for cash distribution
//! - Prepayment and default modeling frameworks (PSA, SDA, CPR, CDR)
//! - Coverage test implementations (OC/IC)
//! - Scenario analysis tools
//!
//! # Module Organization
//!
//! - **Core Types**: Enums, pool, tranches - fundamental data structures
//! - **Waterfall**: Sequential payment distribution with shared factory method
//! - **Behavior Models**: Prepayment, default, and recovery modeling
//! - **Coverage & Risk**: OC/IC tests and structural triggers
//! - **Deal Configuration**: Fees, dates, and default assumptions
//! - **Scenarios**: Stress testing framework
//! - **Utilities**: Rating factors, reinvestment, common trait
//!
//! # Design Philosophy
//!
//! This framework focuses on standard structured credit instruments (90% use case)
//! rather than attempting to model every edge case. Advanced features (PIK bonds,
//! formula-based waterfalls, multiple concurrent waterfalls) are intentionally
//! excluded to maintain clarity and simplicity.

// ============================================================================
// Module Declarations
// ============================================================================

pub mod accounts;
pub mod constants;
pub mod coverage_tests;
pub mod deal_config;
pub mod default_models;
pub mod enums;
pub mod instrument_trait;
pub mod metrics;
pub mod pool;
pub mod prepayment;
pub mod rating_factors;
pub mod reinvestment;
pub mod scenarios;
pub mod serializable_models;
pub mod tranche_valuation;
pub mod tranches;
pub mod waterfall;

// ============================================================================
// Core Type Exports - Fundamental enumerations and classifications
// ============================================================================

pub use enums::{
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
};

// ============================================================================
// Account Management - Deal-level accounts and state management
// ============================================================================

pub use accounts::AccountManager;

// ============================================================================
// Pool & Asset Management - Collateral pool and asset tracking
// ============================================================================

pub use pool::{AssetPool, PoolAsset};

// ============================================================================
// Tranche Structures - Capital structure and credit enhancement
// ============================================================================

pub use tranches::{
    CoverageTrigger, CreditEnhancement, Tranche, TrancheBehaviorType, TrancheBuilder, 
    TrancheCoupon, TrancheStructure,
};

// ============================================================================
// Waterfall Engine - Cash distribution mechanics
// ============================================================================

pub use waterfall::{
    ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRule,
    WaterfallBuilder, WaterfallEngine, WaterfallResult,
};

/// Type alias for backward compatibility with previous waterfall naming
pub type StructuredCreditWaterfall = WaterfallEngine;

// ============================================================================
// Prepayment Models - Voluntary prepayment behavior
// ============================================================================

pub use prepayment::{
    // Factory functions
    calculate_seasoning_months,
    cpr_model,
    cpr_to_smm,
    prepayment_model_for,
    psa_model,
    psa_to_cpr,
    smm_to_cpr,
    vector_model,
    // Core types
    MarketConditions,
    PSAModel, // Commonly used in RMBS
    PrepaymentBehavior,
};

// ============================================================================
// Default & Recovery Models - Credit loss modeling
// ============================================================================

pub use default_models::{
    // Conversion utilities
    cdr_to_mdr,
    // Factory functions
    default_model_for,
    mdr_to_cdr,
    recovery_model_for,
    // Core types
    CDRModel,
    ConstantRecoveryModel,
    CreditFactors,
    DefaultBehavior,
    MarketFactors,
    RecoveryBehavior,
    SDAModel, // Standard Default Assumption
};

// ============================================================================
// Coverage Tests - OC/IC and structural triggers
// ============================================================================

pub use coverage_tests::{
    BreachedTest, CoverageTest, CoverageTests, PaymentDiversion, TestContext, TestResult,
    TestResults,
};

// ============================================================================
// Deal Configuration - Fees, dates, and assumptions
// ============================================================================

pub use deal_config::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};


// ============================================================================
// Scenario Framework - Stress testing and sensitivity analysis
// ============================================================================

pub use scenarios::{
    DefaultScenario, DefaultTimingShape, MarketScenario, PrepaymentScenario, ScenarioComparison,
    ScenarioResult, StructuredCreditScenario,
};

// ============================================================================
// Reinvestment - CLO reinvestment period management
// ============================================================================

pub use reinvestment::{ReinvestmentManager, ReinvestmentTerminationEvent};

// ============================================================================
// Utilities & Common Traits - Shared infrastructure
// ============================================================================

pub use instrument_trait::StructuredCreditInstrument;
pub use rating_factors::{moodys_warf_factor, RatingFactorTable};

// ============================================================================
// Serializable Model Specifications - JSON-friendly wrappers
// ============================================================================

pub use serializable_models::{DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec};

// ============================================================================
// Tranche Valuation - Per-tranche valuation and metrics
// ============================================================================

pub use tranche_valuation::{
    TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
    calculate_tranche_wal, calculate_tranche_duration, calculate_tranche_z_spread,
    calculate_tranche_cs01, register_tranche_metrics,
};
