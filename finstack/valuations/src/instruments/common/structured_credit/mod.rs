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
pub mod scenario_runner;
pub mod scenarios;
pub mod tranches;
pub mod waterfall; // Unified waterfall implementation

// Core enum exports
pub use enums::{
    AssetType, BondType, CardPortfolioType, CreditRating, DealType, LoanType, PaymentMode,
    PropertyType, StudentLoanType, TrancheSeniority, TriggerConsequence, VehicleType,
};

// Other module exports
pub use coverage_tests::{
    BreachedTest, CoverageTests, ICTest, OCTest, PaymentDiversion, TestResults,
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
    cpr_to_smm,
    psa_to_cpr,
    smm_to_cpr,
    MarketConditions,
    PSAModel, // Used by RMBS
    PrepaymentBehavior,
    PrepaymentModelFactory,
};

// Default and recovery models - core types and commonly used models
pub use default_models::{
    cdr_to_mdr, mdr_to_cdr, CDRModel, ConstantRecoveryModel, CreditFactors, DefaultBehavior,
    DefaultModelFactory, MarketFactors, RecoveryBehavior, SDAModel,
};

// Scenario framework
pub use scenario_runner::ScenarioRunner;
pub use scenarios::{
    DefaultScenario, DefaultTimingShape, MarketScenario, PrepaymentScenario, ScenarioBuilder,
    ScenarioComparison, ScenarioLibrary, ScenarioResult, StructuredCreditScenario,
};

// Deal configuration and utilities
pub use deal_config::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};
pub use instrument_trait::{InstrumentDates, InstrumentModels, StructuredCreditInstrument};
pub use rating_factors::{moodys_warf_factor, RatingFactorTable};
