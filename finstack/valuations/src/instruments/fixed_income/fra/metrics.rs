//! FRA and interest rate future metric calculators.
//!
//! Placeholder module to align with the `mod/metrics` layout used by other
//! fixed income instruments. Specific FRA or futures metrics can be added
//! incrementally without changing the module structure.

use crate::metrics::MetricRegistry;

/// Registers FRA and interest rate future metrics.
///
/// Currently no FRA-specific metrics are defined; this function exists to
/// maintain a consistent registration surface across instruments.
pub fn register_fra_metrics(registry: &mut MetricRegistry) {
    let _ = registry;
}
