//! Core margin types and specifications.
//!
//! This module defines the fundamental types for margin and collateral management:
//!
//! - [`CsaSpec`]: Credit Support Annex specification
//! - [`VmParameters`]: Variation margin parameters
//! - [`ImParameters`]: Initial margin parameters
//! - [`EligibleCollateralSchedule`]: Eligible collateral with haircuts
//! - [`MarginCall`]: Margin call event representation
//! - [`OtcMarginSpec`]: OTC derivative margin specification

mod call;
mod collateral;
mod csa;
mod enums;
mod otc;
mod thresholds;

// Re-export all types
pub use call::{MarginCall, MarginCallType};
pub use collateral::{
    CollateralAssetClass, CollateralEligibility, EligibleCollateralSchedule, MaturityConstraints,
};
pub use csa::{CsaSpec, MarginCallTiming};
pub use enums::{ClearingStatus, ImMethodology, MarginFrequency};
pub use otc::OtcMarginSpec;
pub use thresholds::{ImParameters, VmParameters};
