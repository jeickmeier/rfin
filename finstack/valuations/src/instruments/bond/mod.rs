//! Bond instrument module: declares submodules and re-exports types.

pub mod cashflows;
pub mod cashflow_spec;
pub mod metrics;
pub mod pricing;
mod types;

pub use cashflow_spec::CashflowSpec;
pub use types::AmortizationSpec;
pub use types::Bond;
pub use types::CallPut;
pub use types::CallPutSchedule;
