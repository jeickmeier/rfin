
//! Metrics framework for clean separation of pricing and financial measures.
//! 
//! This module provides a trait-based architecture for computing financial
//! metrics independently from core pricing logic. Metrics can be computed
//! on-demand, have dependencies, and are cached for efficiency.
//! 
//! # Key Features
//! 
//! - **Trait-based design**: `MetricCalculator` trait for custom metric implementations
//! - **Dependency management**: Automatic computation ordering based on metric dependencies
//! - **Caching**: Built-in caching of intermediate results like cashflows and discount factors
//! - **Instrument-specific**: Metrics can be registered for specific instrument types
//! - **Standard registry**: Pre-configured registry with common financial metrics
//! 
//! See unit tests and `examples/` for usage.
//! 
//! # Architecture
//! 
//! - **`MetricId`**: Strongly-typed identifiers for all metrics
//! - **`MetricCalculator`**: Trait for implementing custom metrics
//! - **`MetricContext`**: Context containing instrument, market data, and cached results
//! - **`MetricRegistry`**: Registry for managing calculators and dependencies
//! - **Risk metrics**: Specialized calculators for DV01, bucketed risk, and time decay

pub mod ids;
pub mod traits;
pub mod registry;
pub mod risk;

pub use ids::MetricId;
pub use traits::{MetricCalculator, MetricContext};
pub use registry::MetricRegistry;
pub use risk::{BucketedDv01Calculator, BucketSpec, CashflowCaching};

/// Creates a standard metric registry with all built-in metrics.
/// 
/// This registry includes metrics for:
/// - **Bonds**: YTM, duration, convexity, accrued interest, credit spreads
/// - **Interest Rate Swaps**: DV01, annuity factors, par rates
/// - **Deposits**: Discount factors, par rates, year fractions
/// - **Risk**: Bucketed DV01, time decay (theta)
/// 
/// See unit tests and `examples/` for usage.
pub fn standard_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    crate::instruments::fixed_income::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::fixed_income::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::fixed_income::deposit::metrics::register_deposit_metrics(&mut registry);
    crate::instruments::fixed_income::cds::metrics::register_cds_metrics(&mut registry);
    crate::instruments::options::metrics::register_option_metrics(&mut registry);
    crate::instruments::fixed_income::ilb::metrics::register_ilb_metrics(&mut registry);
    risk::register_risk_metrics(&mut registry);
    registry
}