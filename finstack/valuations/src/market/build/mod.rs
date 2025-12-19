//! Quote-to-instrument construction logic.
//!
//! This module provides builders that transform market quotes into concrete instrument instances.
//! Builders resolve conventions, calculate accrual dates, and configure instruments with the
//! appropriate market-standard parameters.
//!
//! # Features
//!
//! - **Rate instruments**: Deposits, FRAs, swaps, and interest rate futures
//! - **Credit instruments**: CDS and CDS tranches with upfront and running spread support
//! - **Build context**: Configurable context with valuation date, notional, and curve mappings
//! - **Prepared quotes**: Envelopes combining quotes with instruments and precomputed pillar times
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::market::build::context::BuildCtx;
//! use finstack_valuations::market::build::rates::build_rate_instrument;
//! use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
//! use finstack_valuations::market::quotes::rates::RateQuote;
//! use finstack_valuations::market::conventions::ids::IndexId;
//! use finstack_core::dates::Date;
//! use std::collections::HashMap;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let ctx = BuildCtx::new(
//!     Date::from_calendar_date(2024, time::Month::January, 2)?,
//!     1_000_000.0,
//!     HashMap::new(),
//! );
//!
//! let quote = RateQuote::Deposit {
//!     id: QuoteId::new("USD-SOFR-DEP-1M"),
//!     index: IndexId::new("USD-SOFR-1M"),
//!     pillar: Pillar::Tenor("1M".parse()?),
//!     rate: 0.0525,
//! };
//!
//! let instrument = build_rate_instrument(&quote, &ctx)?;
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`context::BuildCtx`](context::BuildCtx) for build context configuration
//! - [`prepared::PreparedQuote`](prepared::PreparedQuote) for prepared quote envelopes

/// Builders for credit instruments (CDS).
pub mod cds;
/// Builders for CDS Tranche instruments.
pub mod cds_tranche;
/// Context for building instruments.
pub mod context;
/// Envelope for prepared quotes.
pub mod prepared;
/// Builders for rates instruments.
pub mod rates;
