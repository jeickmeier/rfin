#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Margin, collateral, and XVA (valuation adjustments) framework.
//!
//! This crate provides a standalone home for margin and collateral primitives
//! extracted from `finstack-valuations`.

/// Margin calculation engines.
pub mod calculators;
/// Margin registry/configuration helpers.
pub mod config;
/// Shared margin constants and heuristics.
pub mod constants;
/// Margin-specific analytics and instrument metrics.
pub mod metrics;
/// Embedded registry data and registry wiring.
pub mod registry;
/// Standalone traits used by the margin crate.
pub mod traits;
/// Margin and collateral domain types.
pub mod types;
/// XVA valuation-adjustment models and exposure engines.
pub mod xva;

/// Regulatory capital frameworks (FRTB SBA, SA-CCR).
#[cfg(feature = "regulatory")]
pub mod regulatory;

pub use calculators::im::schedule::{ScheduleAssetClass, BCBS_IOSCO_SCHEDULE_ID};
pub use calculators::im::simm::SimmVersion;
pub use calculators::{
    CcpMarginInputSource, CcpMethodology, ClearingHouseImCalculator, HaircutImCalculator,
    ImCalculator, ImResult, InternalModelImCalculator, InternalModelInputSource,
    ScheduleImCalculator, SimmCalculator, VmCalculator, VmResult,
};
pub use traits::Marginable;
pub use types::{
    ClearingStatus, CollateralAssetClass, CollateralEligibility, ConcentrationBreach, CsaSpec,
    EligibleCollateralSchedule, ImMethodology, ImParameters, InstrumentMarginResult, MarginCall,
    MarginCallTiming, MarginCallType, MarginTenor, MaturityConstraints, NettingSetId,
    OtcMarginSpec, RepoMarginSpec, RepoMarginType, SimmCreditSector, SimmRiskClass,
    SimmSensitivities, VmParameters,
};
