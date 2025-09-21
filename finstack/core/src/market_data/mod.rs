#![allow(clippy::module_name_repetitions)]

//! Market data utilities: curves, surfaces, interpolation utilities and unified
//! trait hierarchy.
//!
//! This module acts as the *public facade* for everything related to market
//! data inside `rustfin-core`.
//!
//! # Sub-modules
//! * [`crate::types::CurveId`] – type-safe identifiers for market data.
//! * [`term_structures`] – one-dimensional term structures such as
//!   [`term_structures::discount_curve::DiscountCurve`],
//!   [`term_structures::forward_curve::ForwardCurve`],
//!   [`term_structures::hazard_curve::HazardCurve`] and
//!   [`term_structures::inflation::InflationCurve`].
//! * [`surfaces`] – two-dimensional objects like implied-volatility surfaces.
//! * Helper functions for interpolation/validation live under
//!   [`crate::math::interp::utils`].
//! * [`context`] – lightweight aggregate of curves, FX, surfaces, and prices.
//!
//! Convenience re-exports are provided so that downstream code can simply
//! `use finstack_core::market_data::*` and obtain the most common symbols.
//!
//! ## Quick-start example
//! ```no_run
//! # use finstack_core::market_data::term_structures::DiscountCurve;
//! # use finstack_core::dates::Date;
//! # use time::Month;
//! # use finstack_core::math::interp::InterpStyle;
//! // 1. Create a simple USD OIS discount curve.
//! let yc = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .unwrap();
//!
//! // 2. Fetch a discount factor 18 months forward.
//! let df = yc.df(1.5);
//! assert!(df < 1.0);
//! ```

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

/// Numeric precision alias re-exported from the surrounding crate so that
/// downstream code can simply `use finstack_core::market_data::F`.
pub use crate::F;
