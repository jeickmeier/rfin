//! Core types for financial statement modeling.

mod model;
mod node;
mod value;

pub use model::{CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec};
pub use node::{ForecastMethod, ForecastSpec, NodeSpec, NodeType, SeasonalMode};
pub use value::AmountOrScalar;
