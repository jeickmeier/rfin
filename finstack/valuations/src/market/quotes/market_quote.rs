//! Unified MarketQuote enum.
//!
//! This module defines the top-level enum for all supported market quotes.

use super::cds::CdsQuote;
use super::cds_tranche::CdsTrancheQuote;
use super::inflation::InflationQuote;
use super::rates::RateQuote;
use super::vol::VolQuote;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Polymorphic container for all supported market quote types.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate instruments
    Rates(RateQuote),
    /// Credit default swaps
    Cds(CdsQuote),
    /// CDS Tranches
    CdsTranche(CdsTrancheQuote),
    /// Inflation instruments
    Inflation(InflationQuote),
    /// Volatility instruments
    Vol(VolQuote),
}

impl MarketQuote {
    /// Bump the quote in its natural market units.
    ///
    /// The `amount` parameter is interpreted per quote type:
    /// - Rates: decimal rate bump (e.g., `0.0001` = 1bp)
    /// - Cds: decimal-to-bp conversion (`spread_bp += amount * 10_000`)
    /// - CdsTranche: decimal-to-bp conversion
    /// - Inflation: decimal rate bump (e.g., `0.0001` = 1bp)
    /// - Vol: absolute vol bump (e.g., `0.01` = +1 vol point)
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            MarketQuote::Rates(q) => MarketQuote::Rates(q.bump(amount)),
            MarketQuote::Cds(q) => MarketQuote::Cds(q.bump(amount)),
            MarketQuote::CdsTranche(q) => MarketQuote::CdsTranche(q.bump(amount)),
            MarketQuote::Inflation(q) => MarketQuote::Inflation(q.bump_rate_decimal(amount)),
            MarketQuote::Vol(q) => MarketQuote::Vol(q.bump_vol_absolute(amount)),
        }
    }
}

/// Trait for filtering quote collections into specific types.
pub trait ExtractQuotes<T> {
    /// Extract all quotes matching type `T` from the collection.
    fn extract_quotes(&self) -> Vec<T>;
}

impl ExtractQuotes<RateQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<RateQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Rates(rq) => Some(rq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CdsQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CdsQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Cds(cq) => Some(cq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CdsTrancheQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CdsTrancheQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::CdsTranche(ctq) => Some(ctq.clone()),
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
