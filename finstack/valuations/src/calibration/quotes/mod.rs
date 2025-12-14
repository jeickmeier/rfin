//! Market quote data structures and types for calibration.
//!
//! This module provides standardized quote schemas for various financial instruments
//! used in curve and surface calibration. Quote types are pure data structures with
//! no pricing logic—pricing is handled by the [`crate::calibration::pricing`] module.
//!
//! # Quote Types
//!
//! - [`RatesQuote`]: Interest rate instruments (deposits, FRAs, futures, swaps)
//! - [`CreditQuote`]: Credit instruments (CDS, tranches)
//! - [`VolQuote`]: Volatility quotes (options, swaptions)
//! - [`InflationQuote`]: Inflation instruments (zero-coupon swaps, YoY swaps)
//! - [`MarketQuote`]: Unified wrapper for all quote types
//!
//! # Conventions
//!
//! Per-instrument conventions can be specified via [`InstrumentConventions`] to override
//! currency defaults for settlement, payment delay, reset lag, and calendar.

mod conventions;
mod credit;
mod inflation;
mod market_quote;
mod rates;
mod vol;

pub use conventions::InstrumentConventions;
pub use credit::CreditQuote;
pub use inflation::InflationQuote;
pub use market_quote::MarketQuote;
pub use rates::{FutureSpecs, RatesQuote};
pub use vol::VolQuote;

