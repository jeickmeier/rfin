//! Swaption (option on interest rate swap) implementation with SABR volatility.
//!
//! Module layout follows simplified instrument code standards:
//! - `metrics/`: per-metric calculators split into files
//! - `parameters.rs`: logical parameter groupings  
//! - `types.rs`: data shape, trait integration, and pricing methods
//! - `pricer.rs`: registry integration pricer

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use types::{Swaption, SwaptionExercise, SwaptionSettlement};
pub use pricer::SimpleSwaptionBlackPricer;
