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
//!     DiscountCurve, ForwardCurve, HazardCurve,
//! };
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::math::interp::InterpStyle;
//! use time::macros::date;
//!
//! let base = date!(2025 - 01 - 01);
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(base)
//!     .knots([(0.0, 1.0), (5.0, 0.88)])
//!     .interp(InterpStyle::MonotoneConvex)
//!     .build()
//!     ?;
//! let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
//!     .base_date(base)
//!     .knots([(0.0, 0.03), (5.0, 0.04)])
//!     .interp(InterpStyle::Linear)
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
mod base_correlation;
/// Internal shared helpers for 1D curves (not exported publicly).
pub(crate) mod common;
/// Credit index aggregates for CDS tranche pricing and credit derivatives.
mod credit_index;
/// Discount factor curves.
mod discount_curve;
/// Flat forward/discount curves.
mod flat;
/// Forward‐rate curves.
mod forward_curve;
/// Credit hazard curves.
mod hazard_curve;
/// Real/Breakeven inflation curves.
mod inflation;
/// Forward price curves (commodities, indices).
mod price_curve;
/// Volatility index forward curves (VIX, VXN, VSTOXX).
mod vol_index_curve;

// Re-export for ergonomic access (curated list)
pub use base_correlation::{
    ArbitrageCheckResult, ArbitrageViolation, BaseCorrelationCurve, BaseCorrelationCurveBuilder,
    SmoothingMethod,
};
pub use credit_index::{CreditIndexData, CreditIndexDataBuilder};
pub use discount_curve::{DiscountCurve, DiscountCurveBuilder, DEFAULT_MIN_FORWARD_TENOR};
pub use flat::FlatCurve;
pub use forward_curve::{ForwardCurve, ForwardCurveBuilder};
pub use hazard_curve::{HazardCurve, HazardCurveBuilder, ParInterp, Seniority};
pub use inflation::{InflationCurve, InflationCurveBuilder};
pub use price_curve::{PriceCurve, PriceCurveBuilder};
pub use vol_index_curve::{VolatilityIndexCurve, VolatilityIndexCurveBuilder};
