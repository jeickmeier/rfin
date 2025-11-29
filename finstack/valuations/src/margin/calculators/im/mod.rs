//! Initial margin calculators.
//!
//! This module provides different IM calculation methodologies:
//!
//! - [`HaircutImCalculator`]: For repos and securities financing
//! - [`SimmCalculator`]: ISDA SIMM for OTC derivatives
//! - [`ScheduleImCalculator`]: BCBS-IOSCO regulatory schedule fallback
//! - [`ClearingHouseImCalculator`]: CCP-specific methodologies

mod clearing;
mod haircut;
mod schedule;
mod simm;

pub use clearing::ClearingHouseImCalculator;
pub use haircut::HaircutImCalculator;
pub use schedule::ScheduleImCalculator;
pub use simm::SimmCalculator;

