//! Forward Rate Agreement (FRA) instrument implementation.
//!
//! FRAs are essential for short-end interest rate curve calibration,
//! providing forward rate fixings between deposit maturities and swap start dates.

mod builder;
pub mod metrics;
mod types;

pub use types::ForwardRateAgreement;

// Provide a distinct path for types.rs to reference this builder
pub(crate) mod mod_fra {
    // builder provided via derive
}
