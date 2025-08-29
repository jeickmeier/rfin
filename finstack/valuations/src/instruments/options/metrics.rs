//! Option metrics registration module
//! 
//! This module provides a unified interface to register all option-specific
//! metrics by delegating to each option type's individual metrics module.

use crate::metrics::MetricRegistry;

/// Register all option metrics with the registry
pub fn register_option_metrics(registry: &mut MetricRegistry) {
    // Register metrics from each option type module
    super::equity_option::metrics::register_equity_option_metrics(registry);
    super::fx_option::metrics::register_fx_option_metrics(registry);
    super::interest_rate_option::metrics::register_interest_rate_option_metrics(registry);
    super::credit_option::metrics::register_credit_option_metrics(registry);
}
