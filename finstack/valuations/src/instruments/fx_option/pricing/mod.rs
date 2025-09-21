//! FX option pricing module.
//!
//! This module contains the pricing engine for `FxOption`, implementing the
//! Garman–Kohlhagen model and helper routines used by metrics. Heavy numerics
//! are kept here to keep the instrument type focused on data shape only.

pub mod engine;

pub use engine::{compute_greeks, FxOptionGreeks, FxOptionPricer};
