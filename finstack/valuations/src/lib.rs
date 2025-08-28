//! Finstack Valuations module.
//! 
//! Provides pricing, risk metrics, and cashflow generation for financial instruments.
//! Built on a metrics framework that separates pricing logic from measure computation.
//! 
//! # Quick Start
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::pricing::result::ValuationResult;
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

pub mod cashflow;
pub mod traits;
pub mod pricing;
pub mod instruments;
pub mod metrics;

pub use finstack_core::prelude::*;
