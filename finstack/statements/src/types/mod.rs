//! Core types for financial statement modeling.

mod node;
mod value;
mod model;

pub use node::{NodeSpec, NodeType};
pub use value::AmountOrScalar;
pub use model::FinancialModelSpec;

