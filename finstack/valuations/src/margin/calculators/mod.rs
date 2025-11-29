//! Margin calculation engines.
//!
//! This module provides calculators for variation margin (VM) and
//! initial margin (IM) following industry standards.
//!
//! # Available Calculators
//!
//! - [`VmCalculator`]: Variation margin calculation per ISDA CSA rules
//! - [`HaircutImCalculator`]: Haircut-based IM for repos
//! - [`SimmCalculator`]: ISDA SIMM for OTC derivatives
//! - [`ScheduleImCalculator`]: BCBS-IOSCO regulatory schedule
//! - [`ClearingHouseImCalculator`]: CCP-specific IM calculation

pub mod im;
mod traits;
mod vm;

// Re-export main types
pub use im::{
    ClearingHouseImCalculator, HaircutImCalculator, ScheduleImCalculator, SimmCalculator,
};
pub use traits::{ImCalculator, ImResult};
pub use vm::{VmCalculator, VmResult};
