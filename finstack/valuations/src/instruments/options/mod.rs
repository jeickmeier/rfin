//! Option instruments including equity, FX, interest rate, and credit options.
//!
//! Provides comprehensive option valuation using Black-Scholes, Garman-Kohlhagen,
//! Black model, and credit option models with full Greeks calculation.

// keep minimal imports; F not directly used here

pub mod cap_floor;
pub mod credit_option;
pub mod equity_option;
pub mod fx_option;
pub mod models;
pub mod swaption;

pub use cap_floor::{InterestRateOption, RateOptionType};
pub use credit_option::CreditOption;
pub use equity_option::EquityOption;
pub use fx_option::FxOption;
pub use models::{BinomialTree, TreeType};
pub use swaption::Swaption;

/// Option type (Call or Put)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    /// Call option (right to buy)
    Call,
    /// Put option (right to sell)
    Put,
}

/// Option exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExerciseStyle {
    /// European option (exercise only at maturity)
    European,
    /// American option (exercise any time before maturity)
    American,
    /// Bermudan option (exercise on specific dates)
    Bermudan,
}

/// Settlement type for options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementType {
    /// Physical delivery of underlying
    Physical,
    /// Cash settlement
    Cash,
}
