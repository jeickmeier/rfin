//! Bond instrument module: declares submodules and re-exports types.

pub mod helpers;
pub mod metrics;
pub mod oas_pricer;
mod types;
pub mod ytm_solver;

pub use types::AmortizationSpec;
pub use types::Bond;
pub use types::CallPut;
pub use types::CallPutSchedule;
