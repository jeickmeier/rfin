//! Adapter modules that implement the mechanics of each `OperationSpec`.
//!
//! Scenario operations are intentionally small data structures. The functions
//! exported from these submodules contain the business logic for applying
//! shocks to market data, financial statements, instruments, and time.

pub mod asset_corr;
pub mod basecorr;
pub mod curves;
pub mod equity;
pub mod fx;
pub mod instruments;
pub mod statements;
pub mod time_roll;
pub(crate) mod traits;
pub mod vol;

pub use time_roll::RollForwardReport;
pub use vol::{check_arbitrage, ArbitrageViolation};
