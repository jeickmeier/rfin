//! Results and export functionality.

#[cfg(feature = "dataframes")]
pub mod export;

// Re-export Results and ResultsMeta from evaluator
pub use crate::evaluator::{Results, ResultsMeta};

// Re-export export functionality when polars feature is enabled
#[cfg(feature = "dataframes")]
pub use export::{to_polars_long, to_polars_long_filtered, to_polars_wide};
