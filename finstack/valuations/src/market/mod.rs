//! Market data inputs, conventions, and quote-to-instrument construction.
//!
//! This module provides the foundation for market data representation and instrument construction
//! in Finstack. It encompasses three main areas:
//!
//! 1. **Market Quotes** (`quotes/`): Stable schemas for market quotes across rates, credit,
//!    inflation, and volatility instruments. Quotes are serializable and include identifiers
//!    for calibration workflows.
//!
//! 2. **Conventions** (`conventions/`): Market convention registries loaded from embedded JSON
//!    data. Conventions define day count, business day adjustments, payment frequencies, and
//!    other market-standard parameters required for instrument construction.
//!
//! 3. **Builders** (`build/`): Quote-to-instrument construction logic that resolves conventions,
//!    calculates dates, and creates concrete instrument instances ready for pricing.
//!
//! # Features
//!
//! - **Stable quote schemas**: All quote types use strict serde names for long-lived pipelines
//! - **Convention registry**: Singleton registry with embedded JSON data for all market conventions
//! - **Quote-to-instrument builders**: Deterministic construction with explicit error handling
//! - **Prepared quotes**: Envelopes combining quotes with instruments and precomputed pillar times
//!   for calibration solvers
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::market::build::context::BuildCtx;
//! use finstack_valuations::market::build::rates::build_rate_instrument;
//! use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
//! use finstack_valuations::market::quotes::rates::RateQuote;
//! use finstack_valuations::market::conventions::ids::IndexId;
//! use finstack_valuations::market::conventions::registry::ConventionRegistry;
//! use finstack_core::dates::Date;
//! use std::collections::HashMap;
//!
//! # fn example() -> finstack_core::Result<()> {
//! // Ensure conventions are loaded
//! let _registry = ConventionRegistry::global();
//!
//! // Create build context
//! let as_of = Date::from_calendar_date(2024, time::Month::January, 2)?;
//! let ctx = BuildCtx::new(as_of, 1_000_000.0, HashMap::new());
//!
//! // Create a deposit quote
//! let quote = RateQuote::Deposit {
//!     id: QuoteId::new("USD-SOFR-DEP-1M"),
//!     index: IndexId::new("USD-SOFR-1M"),
//!     pillar: Pillar::Tenor("1M".parse()?),
//!     rate: 0.0525,
//! };
//!
//! // Build the instrument
//! let instrument = build_rate_instrument(&quote, &ctx)?;
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`build::BuildCtx`](build::context::BuildCtx) for build context configuration
//! - [`conventions::ConventionRegistry`](conventions::registry::ConventionRegistry) for convention lookups
//! - [`quotes::MarketQuote`](quotes::market_quote::MarketQuote) for the unified quote enum

/// Quote-to-instrument builders and prepared quotes.
pub mod build;
/// Market conventions and registries.
pub mod conventions;
/// Market quote schemas.
pub mod quotes;
