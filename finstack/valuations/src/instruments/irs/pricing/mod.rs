//! IRS pricing facade and engine re-export.
//!
//! Provides the pricing entrypoints for `InterestRateSwap`. Core pricing
//! logic lives in `engine`. IRS pricing methods are now included in
//! the Instrument trait via impl_instrument_schedule_pv! macro.

pub mod engine;

pub mod pricer;

// Re-export engine for backward compatibility
pub use engine::IrsEngine;
