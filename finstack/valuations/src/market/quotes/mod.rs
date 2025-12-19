//! Market quote schemas and definitions.
//!
//! This module provides stable, serializable schemas for market quotes across all instrument
//! types. Quotes are designed for calibration workflows and include identifiers, pillars,
//! and market values (rates, spreads, prices, volatilities).
//!
//! # Features
//!
//! - **Stable serde names**: All quote types use strict field names for long-lived pipelines
//! - **Type-safe identifiers**: [`QuoteId`](ids::QuoteId) and convention IDs prevent mismatches
//! - **Pillar support**: Quotes support both tenor-based and date-based maturity pillars
//! - **Bump operations**: Quotes support bumping values for sensitivity calculations
//! - **TypeScript export**: Quotes can be exported to TypeScript when `ts_export` feature is enabled
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
//! use finstack_valuations::market::quotes::rates::RateQuote;
//! use finstack_valuations::market::conventions::ids::IndexId;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let quote = RateQuote::Deposit {
//!     id: QuoteId::new("USD-SOFR-DEP-1M"),
//!     index: IndexId::new("USD-SOFR-1M"),
//!     pillar: Pillar::Tenor("1M".parse()?),
//!     rate: 0.0525,
//! };
//!
//! // Bump the rate by 1 basis point
//! let bumped = quote.bump(0.0001);
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`MarketQuote`](market_quote::MarketQuote) for the unified quote enum
//! - [`ids::QuoteId`](ids::QuoteId) for quote identifiers
//! - [`ids::Pillar`](ids::Pillar) for maturity pillars

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
