//! Deal-type specific metrics for structured credit.

pub mod abs;
pub mod clo_wal;
pub mod cmbs;
pub mod rmbs;

// Re-export ABS metrics
pub use abs::{
    AbsChargeOffCalculator, AbsCreditEnhancementCalculator, AbsDelinquencyCalculator,
    AbsExcessSpreadCalculator, AbsSpeedCalculator,
};

// Re-export CLO metrics
pub use clo_wal::CloWalCalculator;

// Re-export CMBS metrics
pub use cmbs::{CmbsDscrCalculator, CmbsLtvCalculator};

// Re-export RMBS metrics
pub use rmbs::{RmbsFicoCalculator, RmbsLtvCalculator, RmbsWalCalculator};
