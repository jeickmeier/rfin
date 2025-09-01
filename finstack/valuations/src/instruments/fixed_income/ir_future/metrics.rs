//! Interest rate future metric calculators.
//!
//! Placeholder module to align with the `mod/metrics` layout used by other
//! fixed income instruments. Specific future-related metrics can be added later.

use crate::metrics::MetricRegistry;

/// Registers interest rate future metrics.
///
/// Currently no specific metrics are defined; this function exists to
/// maintain a consistent registration surface across instruments.
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    let _ = registry;
}


