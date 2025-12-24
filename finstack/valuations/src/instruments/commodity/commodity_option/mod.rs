//! Commodity option instrument module.
//!
//! Supports European options priced with Black-76 and American options priced
//! via a binomial tree on spot (or futures-implied spot).

/// Metrics for commodity options.
pub mod metrics;
/// Pricer for commodity options.
pub mod pricer;
pub mod traits;
mod types;

pub use pricer::CommodityOptionBlackPricer;
pub use types::CommodityOption;
