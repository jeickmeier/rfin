
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
//! # Quick Start
//! 
//! ```rust
//! use finstack_valuations::metrics::standard_registry;
//! 
//! // Get a registry with all built-in metrics
//! let registry = standard_registry();
//! 
//! // Check available metrics
//! let metrics = registry.available_metrics();
//! assert!(!metrics.is_empty());
//! ```
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
/// # Example
/// ```rust
/// use finstack_valuations::metrics::standard_registry;
/// 
/// let registry = standard_registry();
/// let bond_metrics = registry.metrics_for_instrument("Bond");
/// 
/// // Check that key bond metrics are available
/// assert!(bond_metrics.contains(&finstack_valuations::metrics::MetricId::Ytm));
/// assert!(bond_metrics.contains(&finstack_valuations::metrics::MetricId::DurationMac));
/// ```
pub fn standard_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    risk::register_risk_metrics(&mut registry);
    registry
}