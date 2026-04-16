//! Loss Given Default modeling primitives.
//!
//! Provides seniority-based recovery distributions, collateral-waterfall
//! workout LGD, downturn LGD adjustments, and EAD computation.
//!
//! # Module Organization
//!
//! - [`seniority`]: Beta-distributed recovery by debt seniority class.
//! - [`workout`]: Collateral-first recovery waterfall with costs and
//!   time-to-resolution discounting.
//! - [`downturn`]: Frye-Jacobs and regulatory-floor downturn LGD adjustments.
//! - [`ead`]: Exposure at default with Credit Conversion Factors.

pub mod downturn;
pub mod ead;
pub mod seniority;
pub mod workout;

pub use downturn::{DownturnLgd, DownturnMethod};
pub use ead::{CreditConversionFactor, EadCalculator};
pub use seniority::{BetaRecovery, SeniorityCalibration, SeniorityClass, SeniorityRecovery};
pub use workout::{CollateralPiece, CollateralType, WorkoutCosts, WorkoutLgd, WorkoutLgdBuilder};
