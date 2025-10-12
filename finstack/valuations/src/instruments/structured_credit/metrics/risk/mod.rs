//! Risk and sensitivity metrics for structured credit.

pub mod duration;
pub mod spreads;
pub mod ytm;

pub use duration::{MacaulayDurationCalculator, ModifiedDurationCalculator};
pub use spreads::{Cs01Calculator, SpreadDurationCalculator, ZSpreadCalculator};
pub use ytm::YtmCalculator;
