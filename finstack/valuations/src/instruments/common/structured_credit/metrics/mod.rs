//! Metric calculators for structured credit instruments (CLO, ABS, RMBS, CMBS).
//!
//! This module provides market-standard metrics for structured credit, including:
//! - Accrued interest
//! - Clean/dirty prices
//! - WAL (Weighted Average Life)
//! - Durations (Macaulay, Modified)
//! - Z-spread (consistent across all instruments)
//! - CS01 and spread duration (spread risk)

pub mod accrued;
pub mod duration;
pub mod pool;
pub mod prices;
pub mod spreads;
pub mod wal;
pub mod ytm;

pub use accrued::AccruedCalculator;
pub use duration::{MacaulayDurationCalculator, ModifiedDurationCalculator};
pub use pool::{CdrCalculator, CprCalculator, WamCalculator};
pub use prices::{CleanPriceCalculator, DirtyPriceCalculator};
pub use spreads::{Cs01Calculator, SpreadDurationCalculator, ZSpreadCalculator};
pub use wal::WalCalculator;
pub use ytm::YtmCalculator;

// Note: Metric registration has been moved to individual instrument modules (ABS, CLO, CMBS, RMBS).
// This common module now only exports the shared calculator implementations.
