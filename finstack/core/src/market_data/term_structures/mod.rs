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
//! All curves are bootstrapped from knot points and expose builder helpers to
//! pick an [`crate::market_data::interp::InterpStyle`].  The same
//! interpolation engine therefore underpins *all* term structures.
//!
//! ## Example – building three curves and bundling them in a `MarketContext`
//! ```no_run
//! use finstack_core::market_data::term_structures::*;
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (5.0, 0.88)])
//!     .monotone_convex()
//!     .build()
//!     .unwrap();
//!
//! let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .linear_df()
//!     .build()
//!     .unwrap();
//!
//! let hazard = HazardCurve::builder("USD-CRED")
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     .unwrap();
//!
//! let curves = MarketContext::new()
//!     .with_discount(disc)
//!     .with_forecast(fwd3m)
//!     .with_hazard(hazard);
//! assert!(curves.discount("USD-OIS").is_ok());
//! ```

/// Base correlation curves for CDS tranche pricing.
pub mod base_correlation;
/// Credit curves for risky discounting.
pub mod credit_curve;
/// Discount factor curves.
pub mod discount_curve;
/// Forward‐rate curves.
pub mod forward_curve;
/// Credit hazard curves.
pub mod hazard_curve;
/// Real/Breakeven inflation curves.
pub mod inflation;

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
