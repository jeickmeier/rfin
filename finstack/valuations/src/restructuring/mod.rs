//! Credit event and restructuring analytics toolkit.
//!
//! Provides gone-concern recovery analysis, exchange offer modeling,
//! and liability management exercise (LME) analytics for distressed
//! credit situations.
//!
//! # Modules
//!
//! - **Recovery waterfall**: Distributes enterprise/liquidation value
//!   across ordered claim classes following the Absolute Priority Rule.
//! - **Exchange offers**: Compares hold-vs-tender economics for
//!   distressed exchanges (par-for-par, discount, uptier, downtier).
//! - **LME analysis**: Models open market repurchases, tender offers,
//!   amend-and-extend, and dropdown transactions.
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::restructuring::{
//!     RecoveryWaterfall, Claim, ClaimSeniority, AllocationMode,
//!     execute_recovery_waterfall,
//! };
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//!
//! let waterfall = RecoveryWaterfall {
//!     distributable_value: Money::new(150_000_000.0, Currency::USD),
//!     claims: vec![
//!         Claim {
//!             id: "first_lien".into(),
//!             label: "First Lien Term Loan".into(),
//!             seniority: ClaimSeniority::FirstLienSecured,
//!             principal: Money::new(100_000_000.0, Currency::USD),
//!             accrued_interest: Money::new(2_000_000.0, Currency::USD),
//!             penalties: Money::new(0.0, Currency::USD),
//!             instrument_id: None,
//!             collateral: None,
//!             intra_class_allocation: AllocationMode::ProRata,
//!         },
//!     ],
//!     strict_apr: true,
//!     plan_deviations: vec![],
//! };
//!
//! let result = execute_recovery_waterfall(&waterfall).unwrap();
//! ```
//!
//! # See Also
//!
//! - [`crate::covenants`] for pre-event covenant monitoring
//! - `finstack-statements` for going-concern capital structure

pub(crate) mod error;
pub(crate) mod exchange_offer;
pub(crate) mod lme;
pub(crate) mod recovery_waterfall;
pub(crate) mod types;

// ─── Re-exports ──────────────────────────────────────────────────────

// Core types
pub use types::{AllocationMode, Claim, ClaimSeniority, CollateralAllocation};

// Error
pub use error::RestructuringError;

// Recovery waterfall
pub use recovery_waterfall::{
    execute_recovery_waterfall, ClaimRecovery, PlanDeviation, RecoveryResult, RecoveryWaterfall,
};

// Exchange offers
pub use exchange_offer::{
    analyze_exchange_offer, ConsentTracker, CouponPaymentType, EquityComponentType,
    EquitySweetener, ExchangeInstrument, ExchangeOffer, ExchangeType, HoldVsTenderAnalysis,
    ScenarioEconomics, TenderRecommendation,
};

// LME
pub use lme::{analyze_lme, LeverageImpact, LmeAnalysis, LmeSpec, LmeType, RemainingHolderImpact};
