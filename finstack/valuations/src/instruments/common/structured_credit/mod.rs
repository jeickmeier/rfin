//! Shared building blocks for structured credit instruments.

pub mod coverage_tests;
pub mod coverage_tests_enhanced;
pub mod pool;
pub mod reinvestment;
pub mod tranches;
pub mod types;
pub mod types_extended;
pub mod waterfall;
pub mod waterfall_engine;

// Selective exports to avoid conflicts
pub use coverage_tests::{CoverageTests};
pub use coverage_tests_enhanced::{
    EnhancedCoverageTests, OCTest, ICTest, ParValueTest, DiversityTest,
    WARFTest, WASTest, CoverageTestResults, calculate_all_coverage_tests,
};
pub use pool::{AssetPool, PoolAsset};
pub use reinvestment::{ReinvestmentManager, ReinvestmentTerminationEvent};
pub use tranches::*;
pub use types::{
    DealType, CreditRating, TrancheSeniority, AssetType, LoanType, BondType,
    PropertyType, VehicleType, CardPortfolioType, StudentLoanType,
    PaymentMode, TriggerConsequence,
};
// Re-export CoverageTestType from waterfall_engine to avoid conflict
pub use waterfall_engine::{
    WaterfallEngine, PaymentRule, PaymentRecipient, PaymentCalculation,
    PaymentCondition, DiversionTrigger, ReserveAccount, WaterfallResult,
    PaymentRecord, CoverageRatios, PaymentPriority, ManagementFeeType,
    CoverageTestType,
};
pub use types_extended::{Asset, Tranche, TrancheId, TrancheCoverageTests, CouponType, Seniority};
pub use waterfall::{StructuredCreditWaterfall};
