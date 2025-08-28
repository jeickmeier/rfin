#![deny(missing_docs)]
//! Metrics framework for clean separation of pricing and measures.
//! 
//! This module provides a trait-based architecture for computing financial
//! metrics independently from core pricing logic. Metrics can be computed
//! on-demand, have dependencies, and are cached for efficiency.

pub mod traits;
pub mod registry;

pub use traits::{MetricCalculator, MetricContext, MetricsEnabled};
pub use registry::{MetricRegistry, StandardMetrics};

/// Create a standard metric registry with all built-in metrics.
pub fn standard_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    registry
}