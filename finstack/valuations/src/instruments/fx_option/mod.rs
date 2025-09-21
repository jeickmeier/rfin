//! FX option instrument implementation using Garman–Kohlhagen model.

pub mod metrics;
pub mod parameters;
pub mod pricing;
mod types;

pub use types::{FxOption, FxUnderlyingParams};
