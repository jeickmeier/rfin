//! Core types for financial statement modeling.

mod model;
mod node;
mod value;

pub use model::FinancialModelSpec;
#[cfg(feature = "capital_structure")]
pub use model::{CapitalStructureSpec, DebtInstrumentSpec};
pub use node::{ForecastMethod, ForecastSpec, NodeSpec, NodeType};
pub use value::AmountOrScalar;
