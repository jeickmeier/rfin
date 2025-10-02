//! Core types for financial statement modeling.

mod model;
mod node;
mod value;

pub use model::FinancialModelSpec;
pub use node::{ForecastMethod, ForecastSpec, NodeSpec, NodeType};
pub use value::AmountOrScalar;
