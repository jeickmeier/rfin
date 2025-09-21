//! Inflation swap pricing module.
//!
//! Provides a small pricing facade and engine for zero-coupon inflation swaps.
//! This follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook
//!
//! Public re-exports:
//! - `InflationSwapPricer`: pricing engine with leg PV methods

pub mod engine;

pub use engine::InflationSwapPricer;


