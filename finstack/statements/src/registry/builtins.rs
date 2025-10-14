//! Built-in financial metrics.
//!
//! This module provides access to standard financial metrics that are
//! embedded in the crate. These metrics are stored as JSON files and discovered
//! automatically at runtime.
//!
//! Metrics are organized into namespaces:
//! - `fin.*` - Standard financial metrics
//!   - `fin_basic.json` - Basic metrics (gross_profit, net_income, etc.)
//!   - `fin_margins.json` - Margin calculations
//!   - `fin_returns.json` - Return metrics (ROE, ROA, ROIC, etc.)
//!   - `fin_leverage.json` - Leverage ratios
//!
//! ## Usage
//!
//! Built-in metrics are loaded via [`Registry::load_builtins()`](crate::registry::Registry::load_builtins),
//! which uses `include_str!()` to embed the JSON metric definitions at compile time.
//!
//! ```rust
//! use finstack_statements::registry::Registry;
//!
//! let mut registry = Registry::new();
//! registry.load_builtins()?;
//!
//! // Access metrics from the fin.* namespace
//! assert!(registry.has("fin.gross_profit"));
//! assert!(registry.has("fin.gross_margin"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::error::{Error, Result};
use std::fs;
use std::path::PathBuf;

/// Discover and load all bundled metric registry JSON files.
///
/// Files are located under `data/metrics` (relative to the crate root) and are picked up
/// automatically based on the `.json` extension. The contents are returned in
/// deterministic order (alphabetically by file name).
///
/// This helper is consumed by [`Registry::load_builtins`](crate::registry::Registry::load_builtins).
pub(crate) fn builtin_metric_sources() -> Result<Vec<String>> {
    let metrics_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/metrics");

    let mut discovered: Vec<(String, String)> = fs::read_dir(&metrics_dir)
        .map_err(|e| {
            Error::registry(format!(
                "Failed to read built-in metrics directory '{}': {}",
                metrics_dir.display(),
                e
            ))
        })?
        .filter_map(|entry| match entry {
            Ok(dir_entry) => {
                let path = dir_entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    return None;
                }
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())?;
                match fs::read_to_string(&path) {
                    Ok(contents) => Some(Ok((file_name, contents))),
                    Err(err) => Some(Err(Error::registry(format!(
                        "Failed to read built-in metric file '{}': {}",
                        path.display(),
                        err
                    )))),
                }
            }
            Err(err) => Some(Err(Error::registry(format!(
                "Failed to iterate built-in metrics directory '{}': {}",
                metrics_dir.display(),
                err
            )))),
        })
        .collect::<Result<Vec<_>>>()?;

    // Ensure deterministic ordering regardless of filesystem order
    discovered.sort_by(|a, b| a.0.cmp(&b.0));

    if discovered.is_empty() {
        return Err(Error::registry(format!(
            "No built-in metric JSON files found in '{}'.",
            metrics_dir.display()
        )));
    }

    Ok(discovered
        .into_iter()
        .map(|(_, contents)| contents)
        .collect())
}
