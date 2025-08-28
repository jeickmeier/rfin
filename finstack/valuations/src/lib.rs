#![deny(unsafe_code)]

pub mod cashflow;
pub mod traits;
pub mod pricing;
pub mod instruments;
pub mod risks;
pub mod metrics;

pub use finstack_core::prelude::*;
