//! FX option instrument implementation using Garman–Kohlhagen model.

pub mod pricing;
pub mod metrics;
pub mod parameters;
mod types;

pub use types::{FxOption, FxUnderlyingParams};
