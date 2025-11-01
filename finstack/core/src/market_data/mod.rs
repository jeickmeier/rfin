//! Market data infrastructure for financial instrument pricing.
//!
//! This module provides the core market data types and containers used by
//! pricing engines throughout Finstack. It includes yield curves, credit curves,
//! volatility surfaces, and the [`MarketContext`] that aggregates them for
//! valuation workflows.
//!
//! # Market Data Types
//!
//! ## Term Structures (1D Curves)
//!
//! - **Discount curves** ([`term_structures::DiscountCurve`]): Risk-free rates for discounting
//! - **Forward curves** ([`term_structures::ForwardCurve`]): Expected future interest rates
//! - **Hazard curves** ([`term_structures::HazardCurve`]): Credit default intensities
//! - **Inflation curves** ([`term_structures::InflationCurve`]): CPI expectations
//!
//! ## Surfaces (2D)
//!
//! - **Volatility surfaces** ([`surfaces::VolSurface`]): Implied volatility by strike/maturity
//!
//! ## Scalars and Time Series
//!
//! - **Market scalars** ([`scalars::MarketScalar`]): Spot prices, FX rates, indices
//! - **Inflation indices** ([`scalars::InflationIndex`]): CPI/RPI time series
//!
//! # Market Context
//!
//! [`MarketContext`] aggregates all market data needed for a valuation run:
//! - Stores curves by ID with type-safe retrieval
//! - Provides FX conversion via [`crate::money::fx::FxMatrix`]
//! - Supports scenario bumps and stress testing
//! - Thread-safe for parallel pricing
//!
//! # Industry Standards
//!
//! Market data handling follows conventions from:
//! - **ISDA**: Interest rate and credit curve definitions
//! - **Bloomberg**: Standard curve naming and interpolation
//! - **OpenGamma**: Open source curve construction methodologies
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::market_data::{MarketContext, term_structures::DiscountCurve};
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//!
//! // Build a discount curve
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(base)
//!     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
//!     .build()?;
//!
//! // Add to market context
//! let market = MarketContext::new().insert_discount(curve);
//! assert!(market.get_discount("USD-OIS").is_ok());
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Chapters 4-9 (Market data and curve construction).
//! - Andersen, L., & Piterbarg, V. (2010). *Interest Rate Modeling*.
//!   Volume 1, Chapters 2-4 (Term structure construction).
//! - Rebonato, R. (2004). *Volatility and Correlation* (2nd ed.). Wiley.
//!   (Volatility surface construction and arbitrage)

/// Bump functionality for scenario analysis and stress testing.
pub mod bumps;
/// Market data context with enum-based storage (simplified from V2).
pub mod context;
/// Shared dividend schedules (cash/yield/stock) for equities/ETFs.
pub mod dividends;
/// Scalar market data types and time series (including primitives)
pub mod scalars;
/// Two-dimensional surfaces (e.g. volatility).
pub mod surfaces;
/// One-dimensional term structures (yield, credit, ...).
pub mod term_structures;
/// Public trait hierarchy used by pricing components.
pub mod traits;
// Re-export selected helpers for convenience at `market_data::*` level.
pub use crate::math::interp::utils::validate_knots;
// Re-export MarketContext at the top level for backward compatibility
pub use context::MarketContext;
// Re-export dividend schedule types for convenience
pub use dividends::*;
