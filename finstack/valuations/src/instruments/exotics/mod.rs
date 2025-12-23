//! Exotic and path-dependent options.

/// Asian option module - Average price/strike options.
pub mod asian_option;
/// Barrier option module - Knock-in/knock-out options.
pub mod barrier_option;
/// Basket module - Multi-underlying basket instruments.
pub mod basket;
/// Lookback option module - Path-dependent lookback options.
pub mod lookback_option;

// Re-export primary types
pub use asian_option::{AsianOption, AveragingMethod};
pub use barrier_option::{BarrierOption, BarrierType};
pub use basket::Basket;
pub use lookback_option::{LookbackOption, LookbackType};
