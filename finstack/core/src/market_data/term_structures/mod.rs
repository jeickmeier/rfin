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
//! [`crate::math::interp::InterpStyle`] via a single `set_interp(...)`
//! method on their builders.
//!
//! ## Example – assembling curves inside a `MarketContext`
//! ```rust
//! use finstack_core::market_data::term_structures::{
//!     discount_curve::DiscountCurve,
//!     forward_curve::ForwardCurve,
//!     hazard_curve::HazardCurve,
//! };
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::math::interp::InterpStyle;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(base)
//!     .knots([(0.0, 1.0), (5.0, 0.88)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     .unwrap();
//! let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .base_date(base)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .unwrap();
//! let hazard = HazardCurve::builder("USD-CRED")
//!     .base_date(base)
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     .unwrap();
//!
//! let curves = MarketContext::new()
//!     .insert_discount(disc)
//!     .insert_forward(fwd3m)
//!     .insert_hazard(hazard);
//! assert!(curves.get_discount("USD-OIS").is_ok());
//! ```

/// Base correlation curves for CDS tranche pricing.
pub mod base_correlation;
/// Internal shared helpers for 1D curves (not exported publicly).
pub(crate) mod common;
/// Credit index aggregates for CDS tranche pricing and credit derivatives.
pub mod credit_index;
/// Discount factor curves.
pub mod discount_curve;
/// Forward‐rate curves.
pub mod forward_curve;
/// Credit hazard curves.
pub mod hazard_curve;
/// Real/Breakeven inflation curves.
pub mod inflation;

// Re-export for ergonomic access
pub use base_correlation::*;
pub use credit_index::*;
pub use discount_curve::*;
pub use forward_curve::*;
pub use hazard_curve::*;
pub use inflation::*;
// Re-export the relocated volatility surface
pub use crate::market_data::surfaces::vol_surface::*;
