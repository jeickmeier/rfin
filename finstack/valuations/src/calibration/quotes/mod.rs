//! Market data representation (quotes) for calibration.
//!
//! This module defines the core types for market data inputs, including IR rates,
//! credit spreads, inflation, and volatility. It provides a polymorphic
//! [`MarketQuote`] enum and utilities for filtering and extracting specific
//! data types for calibration steps.
//!
//! # Submodules
//! - [`market_quote`]: The top-level [`MarketQuote`] enum.
//! - [`rates`]: Interest rate instruments (deposits, FRAs, futures, swaps).
//! - [`credit`]: Credit-default swaps (CDS) and tranches.
//! - [`inflation`]: Inflation-linked swaps.
//! - [`vol`]: Volatility instruments (options, swaptions).
//! - [`conventions`]: Instrument-specific pricing conventions.

pub mod conventions;
pub mod credit;
pub mod inflation;
pub(crate) mod json_registry;
pub mod market_quote;
pub mod rate_index;
pub mod rates;
pub mod vol;

pub use conventions::InstrumentConventions;
pub use credit::CreditQuote;
pub use inflation::InflationQuote;
pub use market_quote::MarketQuote;
pub use rates::{FutureSpecs, RatesQuote};
pub use vol::VolQuote;

/// Trait for filtering quote collections into specific types.
///
/// This trait allows callers to extract a subset of specific quote types (e.g., only
/// `RatesQuote`) from a heterogeneous collection of `MarketQuote` objects.
pub trait ExtractQuotes<T> {
    /// Extract all quotes matching type `T` from the collection.
    ///
    /// # Returns
    /// A vector of cloned quotes of type `T`.
    fn extract_quotes(&self) -> Vec<T>;
}

impl ExtractQuotes<RatesQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<RatesQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Rates(rq) => Some(rq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CreditQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CreditQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Credit(cq) => Some(cq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<VolQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<VolQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Vol(vq) => Some(vq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<InflationQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<InflationQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Inflation(iq) => Some(iq.clone()),
                _ => None,
            })
            .collect()
    }
}
