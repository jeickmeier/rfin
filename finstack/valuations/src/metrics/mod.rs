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

pub mod bucketed;
pub mod declarative_registry;
pub mod ids;
pub mod registry;
pub mod traits;

pub use bucketed::{
    standard_ir_dv01_buckets,
    compute_key_rate_dv01_series, compute_key_rate_dv01_series_with_context,
    compute_key_rate_series_for_id, compute_key_rate_series_with_context_for_id,
};
pub use declarative_registry::{
    create_standard_registry as declarative_standard_registry, MetricRegistryBuilder,
};
pub use ids::MetricId;
pub use registry::MetricRegistry;
pub use traits::{MetricCalculator, MetricContext, Structured2D, Structured3D};

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

    crate::instruments::equity::metrics::register_equity_metrics(&mut registry);
    crate::instruments::basket::metrics::register_basket_metrics(&mut registry);
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    crate::instruments::fra::metrics::register_fra_metrics(&mut registry);
    crate::instruments::ir_future::metrics::register_ir_future_metrics(&mut registry);
    crate::instruments::cds::metrics::register_cds_metrics(&mut registry);
    crate::instruments::cds_index::metrics::register_cds_index_metrics(&mut registry);
    crate::instruments::convertible::metrics::register_convertible_metrics(&mut registry);
    crate::instruments::inflation_linked_bond::metrics::register_ilb_metrics(&mut registry);
    crate::instruments::fx_spot::metrics::register_fx_spot_metrics(&mut registry);
    crate::instruments::fx_swap::metrics::register_fx_swap_metrics(&mut registry);
    crate::instruments::inflation_swap::metrics::register_inflation_swap_metrics(&mut registry);
    crate::instruments::equity_option::metrics::register_equity_option_metrics(&mut registry);
    crate::instruments::fx_option::metrics::register_fx_option_metrics(&mut registry);
    crate::instruments::cap_floor::metrics::register_interest_rate_option_metrics(&mut registry);
    crate::instruments::cds_option::metrics::register_cds_option_metrics(&mut registry);
    crate::instruments::swaption::metrics::register_swaption_metrics(&mut registry);
    crate::instruments::repo::metrics::register_repo_metrics(&mut registry);
    crate::instruments::basis_swap::metrics::register_basis_swap_metrics(&mut registry);
    crate::instruments::trs::metrics::register_trs_metrics(&mut registry);
    crate::instruments::variance_swap::metrics::register_variance_swap_metrics(&mut registry);
    crate::instruments::private_markets_fund::register_private_markets_fund_metrics(&mut registry);
    registry
}
