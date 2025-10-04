//! Shared building blocks for structured credit instruments.

pub mod coverage_tests;
pub mod coverage_tests_enhanced;
pub mod default_models;
pub mod pool;
pub mod prepayment;
pub mod reinvestment;
pub mod scenario_runner;
pub mod scenarios;
pub mod tranches;
pub mod types;
pub mod types_extended;
pub mod waterfall;
pub mod waterfall_engine;

// Selective exports to avoid conflicts
pub use coverage_tests::CoverageTests;
pub use coverage_tests_enhanced::{
    calculate_all_coverage_tests, CoverageTestResults, DiversityTest, EnhancedCoverageTests,
    ICTest, OCTest, ParValueTest, WARFTest, WASTest,
};
pub use pool::{AssetPool, PoolAsset};
pub use reinvestment::{ReinvestmentManager, ReinvestmentTerminationEvent};
pub use tranches::*;
pub use types::{
    AssetType, BondType, CardPortfolioType, CreditRating, DealType, LoanType, PaymentMode,
    PropertyType, StudentLoanType, TrancheSeniority, TriggerConsequence, VehicleType,
};
// Re-export CoverageTestType from waterfall_engine to avoid conflict
pub use types_extended::{Asset, CouponType, Seniority, Tranche, TrancheCoverageTests, TrancheId};
pub use waterfall::StructuredCreditWaterfall;
pub use waterfall_engine::{
    CoverageRatios, CoverageTestType, DiversionTrigger, ManagementFeeType, PaymentCalculation,
    PaymentCondition, PaymentPriority, PaymentRecipient, PaymentRecord, PaymentRule,
    ReserveAccount, WaterfallEngine, WaterfallResult,
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
    DefaultScenario, DefaultTimingShape, MarketScenario, PrepaymentScenario,
    ScenarioBuilder, ScenarioComparison, ScenarioLibrary, ScenarioResult,
    StructuredCreditScenario,
};
