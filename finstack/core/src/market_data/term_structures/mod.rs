//! One-dimensional *term structures* – yield curves, forward curves, credit
//! curves and inflation curves.
//!
//! A *term structure* maps a scalar time coordinate (usually expressed in
//! **years** from some base date) to a numerical value such as a **discount
//! factor**, **forward rate** or **survival probability**.  All concrete
//! implementations share the lightweight [`crate::market_data::traits::TermStructure`]
//! super-trait and then extend it with domain-specific behaviour:
//!
//! | Struct                              | Domain  | Specialised trait |
//! |-------------------------------------|---------|-------------------|
//! | [`discount_curve::DiscountCurve`]   | Rates   | [`traits::Discount`](crate::market_data::traits::Discount) |
//! | [`forward_curve::ForwardCurve`]     | Rates   | [`traits::Forward`](crate::market_data::traits::Forward)   |
//! | [`hazard_curve::HazardCurve`]       | Credit  | [`traits::Survival`](crate::market_data::traits::Survival) |
//! | [`base_correlation::BaseCorrelationCurve`] | Credit | (none) |
//! | [`inflation::InflationCurve`]       | CPI     | [`traits::Inflation`](crate::market_data::traits::Inflation) |
//!
//! ## Choosing an interpolation style
//! All curves are bootstrapped from knot points and allow selecting an
//! [`crate::market_data::interp::InterpStyle`] via a single `set_interp(...)`
//! method on their builders.
//!
//! ## Example – building three curves and bundling them in a `MarketContext`
//! ```no_run
//! use finstack_core::market_data::term_structures::*;
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::dates::Date;
//! use time::Month;
//! # use finstack_core::market_data::interp::InterpStyle;
//!
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (5.0, 0.88)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     .unwrap();
//!
//! let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .unwrap();
//!
//! let hazard = HazardCurve::builder("USD-CRED")
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     .unwrap();
//!
//! let curves = MarketContext::new()
//!     .insert_discount(disc)
//!     .insert_forward(fwd3m)
//!     .insert_hazard(hazard);
//! assert!(curves.disc("USD-OIS").is_ok());
//! ```

/// Base correlation curves for CDS tranche pricing.
pub mod base_correlation;
/// Discount factor curves.
pub mod discount_curve;
/// Forward‐rate curves.
pub mod forward_curve;
/// Credit hazard curves.
pub mod hazard_curve;
/// Real/Breakeven inflation curves.
pub mod inflation;
/// Internal shared helpers for 1D curves (not exported publicly).
pub(crate) mod common;

// 2-D surfaces (volatility) now live in market_data::surfaces.

// Unified error type for curve builders (type alias for now).
pub use crate::error::InputError as CurveError;

// Re-export for ergonomic access
pub use base_correlation::*;
pub use discount_curve::*;
pub use forward_curve::*;
pub use hazard_curve::*;
pub use inflation::*;
// Re-export the relocated volatility surface
pub use crate::market_data::surfaces::vol_surface::*;
// Interpolation helpers removed; use `set_interp(InterpStyle::...)` on builders.
