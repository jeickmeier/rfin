#![deny(missing_docs)]
//! Metrics framework for clean separation of pricing and measures.
//! 
//! This module provides a trait-based architecture for computing financial
//! metrics independently from core pricing logic. Metrics can be computed
//! on-demand, have dependencies, and are cached for efficiency.

pub mod traits;
pub mod registry;
pub mod bond_metrics;
pub mod irs_metrics;
pub mod deposit_metrics;

pub use traits::{MetricCalculator, MetricContext, MetricsEnabled};
pub use registry::{MetricRegistry, StandardMetrics};

// Re-export specific calculators for convenience
pub use bond_metrics::{
    AccruedInterestCalculator,
    YtmCalculator,
    MacaulayDurationCalculator,
    ModifiedDurationCalculator,
    ConvexityCalculator,
    YtwCalculator,
};

pub use irs_metrics::{
    AnnuityCalculator,
    ParRateCalculator,
    Dv01Calculator,
    FixedLegPvCalculator,
    FloatLegPvCalculator,
};

/// Create a standard metric registry with all built-in metrics.
pub fn standard_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    bond_metrics::register_bond_metrics(&mut registry);
    irs_metrics::register_irs_metrics(&mut registry);
    deposit_metrics::register_deposit_metrics(&mut registry);
    registry
}
