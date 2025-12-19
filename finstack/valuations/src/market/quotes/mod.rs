//! Market quote schemas and definitions.

/// Credit quotes (CDS).
pub mod cds;
/// CDS tranche quotes.
pub mod cds_tranche;
/// Stable identifiers (QuoteId, Pillar).
pub mod ids;
/// Inflation quotes.
pub mod inflation;
/// Unified MarketQuote enum.
pub mod market_quote;
/// Interest rate quotes (Deposit, FRA, Futures, Swap).
pub mod rates;
/// Volatility quotes.
pub mod vol;
