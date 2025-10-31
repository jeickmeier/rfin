//! Asian option metrics module.

// For now, we just re-export common PV metric
// Future: add delta, vega, gamma, etc. for Asian options

use crate::metrics::MetricRegistry;

/// Register Asian option metrics with the registry.
pub fn register_asian_option_metrics(_registry: &mut MetricRegistry) {
    // Placeholder for future metrics registration
    // For now, PV is handled by the common metrics system
}

