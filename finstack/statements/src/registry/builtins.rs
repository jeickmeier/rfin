//! Built-in financial metrics.
//!
//! This module provides access to standard financial metrics that are
//! bundled with the crate. Metrics are stored as JSON files under
//! `data/metrics` and are exposed via a small helper that is used by
//! [`Registry::load_builtins()`](crate::registry::Registry::load_builtins).
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
//! which uses a platform-appropriate strategy:
//! - On native targets, JSON files are discovered at runtime from the bundled
//!   `data/metrics` directory.
//! - On `wasm32` targets, JSON contents are embedded at compile time via
//!   `include_str!()` so no filesystem access is required.

use crate::error::Result;

// Native-only imports; the wasm32 implementation embeds JSON via `include_str!()`
// and does not touch the filesystem or construct registry errors directly.
#[cfg(not(target_arch = "wasm32"))]
use crate::error::Error;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

/// Discover and load all bundled metric registry JSON files.
///
/// On native targets, files are located under `data/metrics` (relative to the
/// crate root) and are picked up automatically based on the `.json` extension.
/// The contents are returned in deterministic order (alphabetically by file
/// name).
///
/// On `wasm32` targets, filesystem access is not available, so the same set of
/// JSON files is embedded at compile time using `include_str!()`.
///
/// This helper is consumed by [`Registry::load_builtins`](crate::registry::Registry::load_builtins).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn builtin_metric_sources() -> Result<Vec<String>> {
    let metrics_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/metrics");

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

/// Discover and load all bundled metric registry JSON files for `wasm32`.
///
/// On WebAssembly targets, standard filesystem APIs are not available, so we
/// embed the same JSON files at compile time using `include_str!()`. The
/// returned contents are ordered deterministically by file name to match the
/// native implementation.
#[cfg(target_arch = "wasm32")]
pub(crate) fn builtin_metric_sources() -> Result<Vec<String>> {
    // Keep this list in sync with the files under `data/metrics` and with the
    // expectations in the crate documentation.
    let files: &[(&str, &str)] = &[
        (
            "fin_basic.json",
            include_str!("../../data/metrics/fin_basic.json"),
        ),
        (
            "fin_leverage.json",
            include_str!("../../data/metrics/fin_leverage.json"),
        ),
        (
            "fin_margins.json",
            include_str!("../../data/metrics/fin_margins.json"),
        ),
        (
            "fin_returns.json",
            include_str!("../../data/metrics/fin_returns.json"),
        ),
    ];

    let mut discovered: Vec<(String, String)> = files
        .iter()
        .map(|(name, contents)| (name.to_string(), contents.to_string()))
        .collect();

    // Ensure deterministic ordering regardless of list order
    discovered.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(discovered
        .into_iter()
        .map(|(_, contents)| contents)
        .collect())
}
