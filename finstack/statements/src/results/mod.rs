//! Results and export functionality.

#[cfg(feature = "polars_export")]
pub mod export;

// Re-export Results and ResultsMeta from evaluator
pub use crate::evaluator::{Results, ResultsMeta};

// Re-export export functionality when polars feature is enabled
#[cfg(feature = "polars_export")]
pub use export::{to_polars_long, to_polars_wide, to_polars_long_filtered};

