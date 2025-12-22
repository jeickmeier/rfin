//! Margin and collateral management for financial instruments.
//!
//! This module provides industry-standard margining and collateral management
//! following ISDA, BCBS-IOSCO, GMRA, and clearing house standards.
//!
//! # Overview
//!
//! Margining is the process of exchanging collateral to mitigate counterparty credit risk
//! in financial transactions. This module supports:
//!
//! - **Variation Margin (VM)**: Daily mark-to-market payments to eliminate counterparty exposure
//! - **Initial Margin (IM)**: Collateral to cover potential future exposure during close-out
//! - **Collateral Management**: Eligible collateral schedules, haircuts, and substitution
//!
//! # Regulatory Framework
//!
//! | Standard | Scope | Key Requirements |
//! |----------|-------|------------------|
//! | **BCBS-IOSCO** | Bilateral OTC derivatives | VM/IM requirements, eligible collateral |
//! | **ISDA SIMM** | Initial margin calculation | Standardized sensitivities-based IM |
//! | **GMRA 2011** | Repos | Margin maintenance, substitution, haircuts |
//! | **EMIR/Dodd-Frank** | Cleared & uncleared | Daily VM, IM for uncleared |
//!
//! # Module Organization
//!
//! - [`types`]: Core margin types (CSA, collateral, thresholds, margin calls)
//! - [`calculators`]: VM and IM calculation engines
//! - [`metrics`]: Margin-specific risk metrics
//!
//! # Example: Creating a CSA Specification
//!
//! ```rust,no_run
//! use finstack_valuations::margin::{
//!     CsaSpec, VmParameters, ImParameters, ImMethodology,
//!     EligibleCollateralSchedule, MarginTenor,
//! };
//! use finstack_valuations::margin::MarginCallTiming;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//!
//! let csa = CsaSpec {
//!     id: "USD-CSA-STANDARD".to_string(),
//!     base_currency: Currency::USD,
//!     vm_params: VmParameters {
//!         threshold: Money::new(10_000_000.0, Currency::USD),
//!         mta: Money::new(500_000.0, Currency::USD),
//!         rounding: Money::new(10_000.0, Currency::USD),
//!         independent_amount: Money::new(0.0, Currency::USD),
//!         frequency: MarginTenor::Daily,
//!         settlement_lag: 1,
//!     },
//!     im_params: Some(ImParameters {
//!         methodology: ImMethodology::Simm,
//!         mpor_days: 10,
//!         threshold: Money::new(50_000_000.0, Currency::USD),
//!         mta: Money::new(500_000.0, Currency::USD),
//!         segregated: true,
//!     }),
//!     eligible_collateral: EligibleCollateralSchedule::default(),
//!     call_timing: MarginCallTiming::default(),
//!     collateral_curve_id: "USD-OIS".into(),
//! };
//! ```
//!
//! # Industry Standards References
//!
//! - ISDA 2016 Credit Support Annex for Variation Margin (VM CSA)
//! - ISDA 2018 Credit Support Annex for Initial Margin (IM CSA)
//! - BCBS-IOSCO Margin Requirements for Non-Centrally Cleared Derivatives (2015, updated 2020)
//! - ISDA SIMM Methodology v2.6 (2023)
//! - GMRA 2011 (Global Master Repurchase Agreement)

pub mod calculators;
pub mod constants;
mod impls;
pub mod metrics;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use calculators::{
    ClearingHouseImCalculator, HaircutImCalculator, ImCalculator, ImResult, ScheduleImCalculator,
    SimmCalculator, VmCalculator, VmResult,
};
pub use traits::{
    InstrumentMarginResult, Marginable, NettingSetId, SimmRiskClass, SimmSensitivities,
};
pub use types::{
    ClearingStatus, CollateralAssetClass, CollateralEligibility, CsaSpec,
    EligibleCollateralSchedule, ImMethodology, ImParameters, MarginCall, MarginCallTiming,
    MarginCallType, MarginTenor, MaturityConstraints, OtcMarginSpec, VmParameters,
};
