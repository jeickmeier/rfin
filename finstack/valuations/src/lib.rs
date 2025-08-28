#![deny(unsafe_code)]

pub mod cashflow;
pub mod traits;
pub mod pricing;
pub mod instruments;
pub mod risks;
pub mod metrics;

pub use finstack_core::prelude::*;
// Re-export aggregation functions at crate root for backward compatibility
pub use cashflow::aggregation::*;
