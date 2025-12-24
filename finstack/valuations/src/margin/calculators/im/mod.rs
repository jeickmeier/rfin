//! Initial margin calculators.
//!
//! This module provides different IM calculation methodologies:
//!
//! - [`HaircutImCalculator`]: For repos and securities financing
//! - [`SimmCalculator`]: ISDA SIMM for OTC derivatives
//! - [`ScheduleImCalculator`]: BCBS-IOSCO regulatory schedule fallback
//! - [`ClearingHouseImCalculator`]: CCP-specific methodologies
//! - [`InternalModelImCalculator`]: Internal model (VaR/ES-based) stub

mod clearing;
mod haircut;
mod internal;
mod schedule;
mod simm;

pub use clearing::{CcpMarginInputSource, CcpMethodology, ClearingHouseImCalculator};
pub use haircut::HaircutImCalculator;
pub use internal::{InternalModelImCalculator, InternalModelInputSource};
pub use schedule::ScheduleImCalculator;
pub use simm::SimmCalculator;
