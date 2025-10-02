//! Built-in financial metrics.
//!
//! This module provides access to standard financial metrics that are
//! embedded in the crate. These metrics are loaded from JSON files at
//! compile time using `include_str!`.
//!
//! Metrics are organized into namespaces:
//! - `fin.*` - Standard financial metrics
//!   - `fin_basic.json` - Basic metrics (gross_profit, net_income, etc.)
//!   - `fin_margins.json` - Margin calculations
//!   - `fin_returns.json` - Return metrics (ROE, ROA, ROIC, etc.)
//!   - `fin_leverage.json` - Leverage ratios

// This module is currently a placeholder.
// The actual metrics are loaded via Registry::load_builtins()
// which uses include_str!() to embed the JSON files.

