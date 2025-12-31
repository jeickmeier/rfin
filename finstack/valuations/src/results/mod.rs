//! Valuation result types and output formatting.
//!
//! This module provides the result envelope types returned by pricing operations,
//! encapsulating present values, computed metrics, and execution metadata.
//!
//! # Features
//!
//! - **ValuationResult**: Standard result envelope with PV and metrics
//! - **ResultsMeta**: Metadata tracking config, timing, and FX policy
//! - **DataFrame Export**: Convert results to Polars DataFrames
//!
//! # Result Structure
//!
//! Every pricing operation returns a [`ValuationResult`] containing:
//!
//! ```text
//! ValuationResult {
//!     value: Money,              // Present value in instrument currency
//!     measures: HashMap<String, f64>,  // Computed metrics (DV01, Greeks, etc.)
//!     meta: ResultsMeta,         // Execution metadata
//! }
//! ```
//!
//! # Metadata Tracking
//!
//! [`ResultsMeta`] captures important context for audit and reproducibility:
//! - **Timestamp**: When the valuation was computed
//! - **Rounding context**: Numeric precision policy applied
//! - **FX policy**: Currency conversion method (if applicable)
//! - **Parallel flag**: Whether parallel execution was used
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{Bond, Instrument};
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::market_data::context::MarketContext;
//! use time::macros::date;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let bond = Bond::example();
//! let market = MarketContext::new();
//! let as_of = date!(2025-01-15);
//!
//! let result = bond.price_with_metrics(&market, as_of, &[MetricId::Ytm, MetricId::Dv01])?;
//!
//! // Access results
//! println!("PV: {}", result.value);
//! if let Some(dv01) = result.measures.get("dv01") {
//!     println!("DV01: ${:.2}", dv01);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`ValuationResult`] for the main result type
//! - [`ResultsMeta`] for execution metadata
//! - [`crate::metrics`] for available metric calculators

pub mod dataframe;
mod valuation_result;

pub use finstack_core::config::ResultsMeta;
pub use valuation_result::ValuationResult;
