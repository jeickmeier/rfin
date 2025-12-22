//! Term structure curves for rates, credit, and inflation.
//!
//! This module implements one-dimensional term structures that map time to
//! market observables. These curves are fundamental building blocks for pricing
//! fixed income securities and derivatives.
//!
//! # What is a Term Structure?
//!
//! A term structure maps a time coordinate (in years from a base date) to a
//! numerical value representing market expectations:
//! - **Discount factors**: Present value of $1 at time t
//! - **Forward rates**: Expected future interest rates
//! - **Hazard rates**: Credit event intensity (default probability)
//! - **Survival probabilities**: Probability of no default by time t
//! - **Inflation expectations**: Expected CPI growth
//!
//! # Financial Concepts
//!
//! ## Discount Curve
//!
//! The discount curve DF(t) represents the present value of $1 received at time t:
//! ```text
//! DF(t) = e^(-r(t) * t)
//!
//! where r(t) is the continuously compounded zero rate
//! ```
//!
//! ## Forward Curve
//!
//! The forward curve f(t₁, t₂) represents the rate agreed today for borrowing
//! from t₁ to t₂:
//! ```text
//! f(t₁, t₂) = [DF(t₁) / DF(t₂) - 1] / (t₂ - t₁)
//! ```
//!
//! ## Hazard Curve
//!
//! The hazard rate λ(t) represents the instantaneous probability of default:
//! ```text
//! Survival(t) = e^(-∫₀ᵗ λ(s)ds)
//! ```
//!
//! # Curve Types
//!
//! | Struct                        | Domain  | Specialised trait |
//! |-------------------------------|---------|-------------------|
//! | `DiscountCurve`               | Rates   | `Discount`        |
//! | `ForwardCurve`                | Rates   | `Forward`         |
//! | `HazardCurve`                 | Credit  | `Survival`        |
//! | `BaseCorrelationCurve`        | Credit  | (none)            |
//! | `InflationCurve`              | CPI     | `Inflation`       |
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
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::math::interp::InterpStyle;
//! use time::macros::date;
//!
//! let base = date!(2025 - 01 - 01);
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(base)
//!     .knots([(0.0, 1.0), (5.0, 0.88)])
//!     .set_interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     ?;
//! let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .base_date(base)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     ?;
//! let hazard = HazardCurve::builder("USD-CRED")
//!     .base_date(base)
//!     .knots([(0.0, 0.01), (10.0, 0.015)])
//!     .build()
//!     ?;
//!
//! let curves = MarketContext::new()
//!     .insert_discount(disc)
//!     .insert_forward(fwd3m)
//!     .insert_hazard(hazard);
//! assert!(curves.get_discount("USD-OIS").is_ok());
//! # Ok::<(), finstack_core::Error>(())
//! ```

/// Base correlation curves for CDS tranche pricing.
pub mod base_correlation;
/// Internal shared helpers for 1D curves (not exported publicly).
pub(crate) mod common;
/// Credit index aggregates for CDS tranche pricing and credit derivatives.
pub mod credit_index;
/// Discount factor curves.
pub mod discount_curve;
/// Flat forward/discount curves.
pub mod flat;
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
pub use flat::*;
pub use forward_curve::*;
pub use hazard_curve::*;
pub use inflation::*;
// Re-export the relocated volatility surface
pub use crate::market_data::surfaces::vol_surface::*;
