//! Adapters for applying shocks to market data and statements.

pub mod basecorr;
pub mod curves;
pub mod equity;
pub mod fx;
pub mod instruments;
pub mod statements;
pub mod time_roll;
pub mod vol;

pub use time_roll::RollForwardReport;
