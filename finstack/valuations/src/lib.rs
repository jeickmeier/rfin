//! Finstack Valuations module.
//!
//! Provides pricing, risk metrics, and cashflow generation for financial instruments.
//! Built on a metrics framework that separates pricing logic from measure computation.
//!
//! # Quick Start
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::results::ValuationResult;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention};
//! use finstack_core::dates::StubKind;
//! use time::Month;
//!
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//! // Note: Bond constructor would be used here
//! ```

pub mod calibration;
pub mod cashflow;
pub mod results;

// Export macros before instruments module
#[macro_use]
pub mod instruments;
pub mod covenants;
pub mod metrics;
pub mod performance;

// Optional: allow enabling pricer registration behind a non-default feature flag.
// Disabled by default to avoid unexpected-cfg lints.
// When enabled, this runs at crate init to register default pricers.
#[allow(unused)]
fn __maybe_register_default_pricers() {
    #[cfg(any())]
    {
        crate::instruments::bond::pricing::register_default_bond_pricers();
    }
}

pub use finstack_core::prelude::*;

// Optional: callers may invoke this to install default pricers for instruments.
// We avoid automatic registration to keep binary init predictable and reduce
// surprising global state.
pub use crate::instruments::install_default_pricers as install_pricers;

// Example usage (pseudo-code): register default pricers and toggle models by key.
