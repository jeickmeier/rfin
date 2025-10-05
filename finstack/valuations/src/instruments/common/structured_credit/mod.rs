//! Shared building blocks for structured credit instruments.

pub mod coverage_tests;
pub mod deal_config;
pub mod default_models;
pub mod instrument_trait;
pub mod pool;
pub mod prepayment;
pub mod rating_factors;
pub mod reinvestment;
pub mod scenario_runner;
pub mod scenarios;
pub mod tranches;
pub mod enums;
pub mod models;
pub mod waterfall; // Unified waterfall implementation

// Core enum exports
pub use enums::{
    AssetType, BondType, CardPortfolioType, CreditRating, DealType, LoanType, PaymentMode,
    PropertyType, StudentLoanType, TrancheSeniority, TriggerConsequence, VehicleType,
};

// Data model exports
pub use models::{Asset, CouponType, Seniority, Tranche, TrancheCoverageTests, TrancheId};

// Other module exports
pub use coverage_tests::{
    BreachedTest, CoverageTestEngine, CoverageTests, ICTest, OCTest, PaymentDiversion, TestResults,
};
pub use pool::{AssetPool, PoolAsset};
pub use reinvestment::{ReinvestmentManager, ReinvestmentTerminationEvent};
pub use tranches::*;

// Export unified waterfall types
pub use waterfall::{
    CoverageTestType, DiversionTrigger, FeeBase, ManagementFeeType, PaymentCalculation,
    PaymentCondition, PaymentDetail, PaymentRecipient, PaymentRecord, PaymentRule,
    PrincipalPaymentType, ReserveAccount, StructuredCreditWaterfall, WaterfallAllocation,
    WaterfallBuilder, WaterfallEngine, WaterfallResult, WaterfallStep,
};

// Prepayment models
pub use prepayment::{
    calculate_seasoning_months, cpr_to_smm, psa_to_cpr, smm_to_cpr, AutoPrepaymentModel, CPRModel,
    CommercialPrepaymentModel, CreditCardPaymentModel, MarketConditions, MortgagePrepaymentModel,
    PSAModel, PrepaymentBehavior, PrepaymentModelFactory, StudentLoanPrepaymentModel, VectorModel,
};

// Default and recovery models
pub use default_models::{
    cdr_to_mdr, mdr_to_cdr, AutoDefaultModel, CDRModel, CollateralRecoveryModel,
    ConstantRecoveryModel, CreditCardChargeOffModel, CreditFactors, DefaultBehavior,
    DefaultModelFactory, MarketFactors, MortgageDefaultModel, RecoveryBehavior, SDAModel,
    VectorDefaultModel,
};

// Scenario framework
pub use scenario_runner::ScenarioRunner;
pub use scenarios::{
    DefaultScenario, DefaultTimingShape, MarketScenario, PrepaymentScenario, ScenarioBuilder,
    ScenarioComparison, ScenarioLibrary, ScenarioResult, StructuredCreditScenario,
};

// Deal configuration and utilities
pub use deal_config::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};
pub use instrument_trait::StructuredCreditInstrument;
pub use rating_factors::{moodys_warf_factor, RatingFactorTable};
