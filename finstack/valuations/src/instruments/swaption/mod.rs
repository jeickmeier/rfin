//! Swaption (option on interest rate swap) implementation with SABR volatility.
//!
//! Module layout follows instrument code standards:
//! - `pricing/`: pricing entrypoints and engines (Black, SABR)
//! - `metrics/`: per-metric calculators split into files
//! - `parameters.rs`: logical parameter groupings
//! - `types.rs`: data shape and trait integration

pub mod metrics;
pub mod parameters;
pub mod pricing;
mod types;

pub use types::{Swaption, SwaptionExercise, SwaptionSettlement};
