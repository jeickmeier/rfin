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
pub mod vol;

pub use asset_corr::{
    apply_asset_correlation_shock, apply_prepay_default_correlation_shock,
    apply_selective_correlation_shock,
};
pub use time_roll::RollForwardReport;
pub use vol::ArbitrageViolation;
