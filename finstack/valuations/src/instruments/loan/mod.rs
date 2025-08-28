//! Private credit loan instruments.
//!
//! Supports term loans with complex features including:
//! - Time-varying rate schedules (fixed/floating with step-ups)
//! - Cash/PIK/Toggle coupon structures  
//! - Complex amortization (bullet, linear, fixed %, custom schedules)
//! - Call schedules and prepayment options with penalties
//! - Multiple fee types (origination, commitment, amendment, exit)

pub mod term_loan;
pub mod prepayment;
pub mod covenants;
pub mod ddtl;
pub mod revolver;

pub use term_loan::{Loan, InterestSpec};
pub use prepayment::{PrepaymentSchedule, PrepaymentType, PrepaymentPenalty, PenaltyType};
pub use covenants::{Covenant, CovenantType, CovenantConsequence};
pub use ddtl::{DelayedDrawTermLoan, DrawEvent, DrawRules};
pub use revolver::{RevolvingCreditFacility, UtilizationFeeSchedule};
