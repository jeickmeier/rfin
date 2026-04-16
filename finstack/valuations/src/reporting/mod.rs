//! Structured report generation components for analytics results.
//!
//! This module provides composable, self-contained data components that wrap
//! existing analytics results ([`ValuationResult`], [`SensitivityMatrix`], etc.)
//! and present them in formats suitable for downstream rendering: JSON dicts,
//! Markdown tables, and DataFrames.
//!
//! Each component is independent -- consumers compose them into dashboards,
//! notebooks, or API responses as they see fit. No rendering dependencies
//! are introduced; components produce data, not pixels.
//!
//! # Components
//!
//! - [`MetricsTable`]: Key-value pairs of computed metrics from a [`ValuationResult`]
//! - [`SensitivityGrid`]: 2D grid of bucketed sensitivities from a [`SensitivityMatrix`]
//! - [`CashflowLadder`]: Time-bucketed cashflow summary
//! - [`ScenarioMatrix`]: Scenario name x metric matrix
//! - [`WaterfallData`]: Ordered steps for waterfall chart visualization
//!
//! # Formatting Utilities
//!
//! The [`format`] sub-module provides pure functions for number formatting
//! (`format_bps`, `format_pct`, `format_currency`, `format_ratio`). Components
//! do not call these internally -- they are exposed separately for consumers
//! who want formatted display strings.
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::reporting::{WaterfallData, ReportComponent};
//!
//! let waterfall = WaterfallData::from_attribution(
//!     "P&L Attribution",
//!     "USD",
//!     1_000_000.0,
//!     1_050_000.0,
//!     &[
//!         ("Rates".to_string(), 30_000.0),
//!         ("Credit".to_string(), 20_000.0),
//!     ],
//! );
//!
//! // Structured JSON for downstream rendering
//! let json = waterfall.to_json();
//! assert_eq!(json["title"], "P&L Attribution");
//!
//! // Markdown for notebook display
//! let md = waterfall.to_markdown();
//! assert!(md.contains("Rates"));
//! ```

mod cashflow_ladder;
pub mod format;
mod metrics_table;
mod scenario_matrix;
mod sensitivity_grid;
mod waterfall;

pub use cashflow_ladder::{BucketFrequency, CashflowBucket, CashflowLadder};
pub use metrics_table::{Direction, MetricRow, MetricUnit, MetricsTable};
pub use scenario_matrix::ScenarioMatrix;
pub use sensitivity_grid::SensitivityGrid;
pub use waterfall::{WaterfallData, WaterfallStep};

/// Trait for structured report components that can be serialized to
/// multiple output formats. Each component wraps an analytics result
/// and presents it as a structured data object.
pub trait ReportComponent: Send + Sync {
    /// Serialize to a JSON value ([`serde_json::Value`]).
    ///
    /// This is the canonical structured output used by both
    /// Python (`.to_dict()`) and WASM (`.to_json()`) bindings.
    fn to_json(&self) -> serde_json::Value;

    /// Render as a Markdown string (table, list, or prose).
    fn to_markdown(&self) -> String;

    /// Component type name for dispatch and labeling.
    fn component_type(&self) -> &'static str;
}
