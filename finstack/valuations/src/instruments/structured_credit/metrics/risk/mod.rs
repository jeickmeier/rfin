//! Risk and sensitivity metrics for structured credit.

pub mod default01;
pub mod duration;
pub mod prepayment01;
pub mod recovery01;
pub mod severity01;
pub mod spreads;
pub mod ytm;

pub use default01::Default01Calculator;
pub use duration::{
    calculate_tranche_duration, MacaulayDurationCalculator, ModifiedDurationCalculator,
};
pub use prepayment01::Prepayment01Calculator;
pub use severity01::Severity01Calculator;
pub use spreads::{
    calculate_tranche_cs01, calculate_tranche_z_spread, Cs01Calculator, SpreadDurationCalculator,
    ZSpreadCalculator,
};
pub use ytm::YtmCalculator;
