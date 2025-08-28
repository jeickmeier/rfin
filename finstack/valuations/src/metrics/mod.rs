#![deny(missing_docs)]
//! Metrics framework for clean separation of pricing and measures.
//! 
//! This module provides a trait-based architecture for computing financial
//! metrics independently from core pricing logic. Metrics can be computed
//! on-demand, have dependencies, and are cached for efficiency.

pub mod ids;
pub mod traits;
pub mod registry;
pub mod risk;

pub use ids::MetricId;
pub use traits::{MetricCalculator, MetricContext, InstrumentData, MarketData, ComputationCache};
pub use registry::{MetricRegistry, StandardMetrics};
pub use risk::{BucketedDv01Calculator, BucketSpec, CashflowCaching};

/// Create a standard metric registry with all built-in metrics.
pub fn standard_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    risk::register_risk_metrics(&mut registry);
    registry
}