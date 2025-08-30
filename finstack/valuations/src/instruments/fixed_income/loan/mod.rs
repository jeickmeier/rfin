//! Private credit loan instruments.
//!
//! Supports term loans with complex features including:
//! - Time-varying rate schedules (fixed/floating with step-ups)
//! - Cash/PIK/Toggle coupon structures  
//! - Complex amortization (bullet, linear, fixed %, custom schedules)
//! - Call schedules and prepayment options with penalties
//! - Multiple fee types (origination, commitment, amendment, exit)

pub mod covenants;
pub mod ddtl;
pub mod prepayment;
pub mod revolver;
pub mod term_loan;

pub use covenants::{Covenant, CovenantConsequence, CovenantType};
pub use ddtl::{DelayedDrawTermLoan, DrawEvent, DrawRules};
pub use prepayment::{PenaltyType, PrepaymentPenalty, PrepaymentSchedule, PrepaymentType};
pub use revolver::{RevolvingCreditFacility, UtilizationFeeSchedule};
pub use term_loan::{InterestSpec, Loan};
