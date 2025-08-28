#![allow(missing_docs)]
pub mod irs;
pub mod bond;
pub mod deposit;

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

impl Instrument {
    /// Return the instrument type as a string identifier.
    /// 
    /// This centralizes instrument type detection logic and eliminates
    /// repeated match statements throughout the codebase.
    pub fn instrument_type(&self) -> &'static str {
        match self {
            Instrument::Bond(_) => "Bond",
            Instrument::IRS(_) => "IRS",
            Instrument::Deposit(_) => "Deposit",
        }
    }
}


