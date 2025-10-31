//! Autocallable structured product instrument module.

pub mod metrics;
pub mod pricer;
pub mod traits;
pub mod types;

pub use types::{Autocallable, FinalPayoffType};
