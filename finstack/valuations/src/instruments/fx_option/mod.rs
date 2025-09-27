//! FX option instrument implementation using Garman–Kohlhagen model.

pub mod calculator;
pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use crate::instruments::common::parameters::FxUnderlyingParams;
pub use calculator::{FxOptionCalculator, FxOptionGreeks};
pub use pricer::SimpleFxOptionBlackPricer;
pub use types::FxOption;
