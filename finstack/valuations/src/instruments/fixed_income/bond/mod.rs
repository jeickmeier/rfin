//! Bond instrument module: declares submodules and re-exports types.

pub mod metrics;
pub mod pricing;
pub mod cashflows;
mod types;

pub use types::AmortizationSpec;
pub use types::Bond;
pub use types::CallPut;
pub use types::CallPutSchedule;
