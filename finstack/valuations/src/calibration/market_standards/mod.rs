//! Market-standard conventions and registries for calibration.
//!
//! This module provides currency-specific defaults and rate index classification
//! used by calibration and pricing logic. It separates "market knowledge" from
//! quote schemas and pricing mechanics.
//!
//! # Currency Defaults
//!
//! - [`settlement_days_for_currency`]: T+0/T+1/T+2 conventions
//! - [`standard_day_count_for_currency`]: ACT/360 vs ACT/365F by currency
//! - [`default_calendar_for_currency`]: Settlement calendar IDs
//!
//! # Rate Index Registry
//!
//! - [`RateIndexFamily`]: Overnight vs Term rate classification
//! - [`lookup_index_info`]: Registry lookup for rate indices
//! - [`is_overnight_index`]: Quick overnight rate check
//! - [`ois_compounding_for_index`]: OIS compounding method by index/currency
//!
//! # Swaption Conventions
//!
//! - [`SwaptionMarketConvention`]: Currency-specific swaption market conventions

mod currency_defaults;
mod rates_index_registry;
mod swaption_conventions;

pub use currency_defaults::{
    default_calendar_for_currency, settlement_days_for_currency, standard_day_count_for_currency,
};
pub use rates_index_registry::{
    is_overnight_index, lookup_index_info, ois_compounding_for_index, RateIndexFamily,
    RateIndexInfo,
};
pub use swaption_conventions::{PaymentEstimation, SwaptionMarketConvention};

