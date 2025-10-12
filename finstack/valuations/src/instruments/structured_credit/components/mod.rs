//! Components for structured credit instruments.
//!
//! This module contains all the building blocks for structured credit:
//! - Structural components: enums, pool, tranches, waterfall
//! - Behavioral models: prepayment, default, recovery
//! - Valuation: tranche-specific cashflow and pricing functions

// Structural components
pub mod enums;
pub mod pool;
pub mod tranches;
pub mod waterfall;

// Behavioral models
pub mod prepayment;
pub mod default_models;
pub mod serializable;

// Valuation
pub mod tranche_valuation;

// ============================================================================
// Re-export structural components
// ============================================================================

pub use enums::{
    AssetType, CreditRating, DealType, PaymentMode, TrancheSeniority, TriggerConsequence,
};

pub use pool::{
    AssetPool, PoolAsset, PoolStats, ReinvestmentPeriod, ReinvestmentCriteria,
    ConcentrationCheckResult, ConcentrationViolation,
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

// ============================================================================
// Re-export behavioral models
// ============================================================================

pub use prepayment::{
    PrepaymentBehavior, PSAModel, CPRModel, VectorModel, AnnualStepCprModel,
    MarketConditions, calculate_seasoning_months, cpr_to_smm, smm_to_cpr,
    psa_to_cpr, prepayment_model_for, psa_model, cpr_model, vector_model,
};

pub use default_models::{
    DefaultBehavior, RecoveryBehavior, CDRModel, SDAModel, VectorDefaultModel,
    MortgageDefaultModel, AutoDefaultModel, CreditCardChargeOffModel,
    ConstantRecoveryModel, CollateralRecoveryModel, AnnualStepCdrModel,
    CreditFactors, MarketFactors, cdr_to_mdr, mdr_to_cdr,
    default_model_for, recovery_model_for,
};

pub use serializable::{
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
};

// ============================================================================
// Re-export tranche valuation
// ============================================================================

pub use tranche_valuation::{
    TrancheCashflowResult, TrancheValuation, TrancheValuationExt,
    calculate_tranche_wal, calculate_tranche_duration,
    calculate_tranche_z_spread, calculate_tranche_cs01,
};
