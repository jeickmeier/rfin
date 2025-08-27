#![allow(clippy::module_name_repetitions)]

//! Market data utilities: curves, surfaces, interpolation utilities and unified
//! trait hierarchy.
//!
//! This module acts as the *public facade* for everything related to market
//! data inside `rustfin-core`.
//!
//! # Sub-modules
//! * [`id`] – lightweight, zero-cost identifiers such as [`CurveId`].
//! * [`interp`] – a collection of curve interpolation schemes implementing the
//!   polymorphic [`interp::InterpFn`] trait.
//! * [`term_structures`] – one-dimensional term structures such as
//!   [`term_structures::DiscountCurve`], [`term_structures::ForwardCurve`],
//!   [`term_structures::HazardCurve`] and [`term_structures::InflationCurve`].
//! * [`surfaces`] – two-dimensional objects like implied-volatility surfaces.
//! * [`utils`] – helper functions shared across the implementation
//!   (validation, segment location, etc.).
//! * [`multicurve`] – thin container for keeping many curves in one place.
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
//! // 1. Create a simple USD OIS discount curve.
//! let yc = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
//!     .linear_df()
//!     .build()
//!     .unwrap();
//!
//! // 2. Fetch a discount factor 18 months forward.
//! let df = yc.df(1.5);
//! assert!(df < 1.0);
//! ```

/// Identifier utilities (see [`id::CurveId`]).
pub mod id;
/// Inflation index data (CPI/RPI) using Polars DataFrames.
pub mod inflation_index;
/// Interpolation framework and concrete algorithms.
pub mod interp;
/// Two-dimensional surfaces (e.g. volatility).
pub mod surfaces;
/// One-dimensional term structures (yield, credit, ...).
pub mod term_structures;
/// Generic market primitives: scalars and time series
pub mod primitives;
/// Public trait hierarchy used by pricing components.
pub mod traits;
/// Helper validation utilities shared across market-data code.
pub mod utils;
/// Unified market-data context for valuations.
pub mod context;
// Re-export helper(s)
pub use utils::validate_knots;

// Re-export common term structures at the market_data::* level for backwards compatibility.
pub use term_structures::{discount_curve, forward_curve, hazard_curve, inflation};
// Re-export volatility surface from the new module for unchanged public path `market_data::vol_surface`.
pub use surfaces::vol_surface;
// Also re-export the concrete VolSurface type for a shorter import path.
pub use surfaces::vol_surface::VolSurface;
// Re-export context types
pub use context::MarketContext;

pub mod trees {
    //! Tree-based lattice structures (option, rate, credit).
    pub mod credit_tree;
    pub mod option_tree;
    pub mod rate_tree;

    // Re-export aliases – suppress unused warnings for now
    #[allow(unused_imports)]
    pub use credit_tree::*;
    #[allow(unused_imports)]
    pub use option_tree::*;
    #[allow(unused_imports)]
    pub use rate_tree::*;
}

// Backwards re-export at market_data::* level
pub use trees::{credit_tree, option_tree, rate_tree};

pub mod multicurve;

/// Numeric precision alias re-exported from the surrounding crate so that
/// downstream code can simply `use finstack_core::market_data::F`.
pub use crate::F;
