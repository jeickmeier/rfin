#![allow(missing_docs)]

pub mod deposit;
pub mod irs;
pub mod bond;

pub use bond::Bond;
pub use deposit::Deposit;
pub use irs::InterestRateSwap;

/// A concrete enum for all supported instrument types.

#[derive(Clone, Debug)]
pub enum Instrument {
    /// Fixed-rate bond instrument
    Bond(Bond),
    /// Interest rate swap instrument
    IRS(InterestRateSwap),
    /// Deposit instrument
    Deposit(Deposit),
}


