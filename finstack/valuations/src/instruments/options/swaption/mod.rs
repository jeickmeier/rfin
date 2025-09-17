//! Swaption (option on interest rate swap) implementation with SABR volatility.

pub mod metrics;
mod types;
pub mod parameters;

pub use types::{Swaption, SwaptionExercise, SwaptionSettlement};
