//! Interest rate option pricing facade and engine re-export.
//!
//! Exposes the pricing entrypoints for `InterestRateOption`. Core pricing
//! logic lives in `engine`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.

pub mod black;
pub mod engine;

pub use engine::IrOptionPricer;
