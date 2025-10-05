//! Shared building blocks for structured credit instruments.

pub mod coverage_tests;
pub mod deal_config;
pub mod default_models;
pub mod enums;
pub mod instrument_trait;
pub mod pool;
pub mod prepayment;
pub mod rating_factors;
pub mod reinvestment;
pub mod scenarios;
pub mod tranches;
pub mod waterfall; // Unified waterfall implementation

// Core enum exports
pub use enums::{
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
};

// Other module exports
#[allow(deprecated)]
pub use coverage_tests::{
    BreachedTest,
    CoverageTest,
    CoverageTests,
    // Legacy types (deprecated)
    ICTest,
    OCTest,
    PaymentDiversion,
    TestContext,
    TestResult,
    TestResults,
};
pub use pool::{AssetPool, PoolAsset};
pub use reinvestment::{ReinvestmentManager, ReinvestmentTerminationEvent};
pub use tranches::*;

// Waterfall - core types
pub use waterfall::{
    ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRule,
    StructuredCreditWaterfall, WaterfallBuilder, WaterfallEngine, WaterfallResult,
};

// Prepayment models - core types and commonly used models
pub use prepayment::{
    calculate_seasoning_months,
    cpr_model,
    cpr_to_smm,
    prepayment_model_for,
    psa_model,
    psa_to_cpr,
    smm_to_cpr,
    vector_model,
    MarketConditions,
    PSAModel, // Used by RMBS
    PrepaymentBehavior,
};

// Default and recovery models - core types and commonly used models
pub use default_models::{
    cdr_to_mdr, default_model_for, mdr_to_cdr, recovery_model_for, CDRModel, ConstantRecoveryModel,
    CreditFactors, DefaultBehavior, MarketFactors, RecoveryBehavior, SDAModel,
};

// Scenario framework
pub use scenarios::{
    DefaultScenario, DefaultTimingShape, MarketScenario, PrepaymentScenario, ScenarioComparison,
    ScenarioResult, StructuredCreditScenario,
};

// Deal configuration and utilities
pub use deal_config::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};
pub use instrument_trait::{InstrumentDates, InstrumentModels, StructuredCreditInstrument};
pub use rating_factors::{moodys_warf_factor, RatingFactorTable};
