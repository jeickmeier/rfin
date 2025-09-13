//! Deposit instrument implementation.

mod builder;
pub mod metrics;
mod types;

pub use types::Deposit;

// Provide a distinct path for types.rs to reference this builder
#[allow(unused_imports)]
pub(crate) mod mod_deposit {
    pub use super::builder::DepositBuilder;
}
