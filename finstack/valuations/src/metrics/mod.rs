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
pub mod registry;
pub mod risk;
pub mod traits;
pub mod traits_ext;

pub use ids::MetricId;
pub use registry::MetricRegistry;
pub use risk::{BucketSpec, BucketedDv01Calculator, CashflowCaching};
pub use traits::{MetricCalculator, MetricContext};
pub use traits_ext::{RiskBucket, RiskMeasurable, RiskReport};

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
    crate::instruments::fixed_income::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::fixed_income::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::fixed_income::deposit::metrics::register_deposit_metrics(&mut registry);
    crate::instruments::fixed_income::fra::metrics::register_fra_metrics(&mut registry);
    crate::instruments::fixed_income::ir_future::metrics::register_ir_future_metrics(&mut registry);
    crate::instruments::fixed_income::cds::metrics::register_cds_metrics(&mut registry);
    crate::instruments::fixed_income::cds_index::metrics::register_cds_index_metrics(&mut registry);
    crate::instruments::fixed_income::convertible::metrics::register_convertible_metrics(
        &mut registry,
    );
    crate::instruments::fixed_income::inflation_linked_bond::metrics::register_ilb_metrics(
        &mut registry,
    );
    crate::instruments::fixed_income::fx_spot::metrics::register_fx_spot_metrics(&mut registry);
    crate::instruments::fixed_income::fx_swap::metrics::register_fx_swap_metrics(&mut registry);
    crate::instruments::fixed_income::inflation_swap::metrics::register_inflation_swap_metrics(
        &mut registry,
    );
    crate::instruments::options::equity_option::metrics::register_equity_option_metrics(
        &mut registry,
    );
    crate::instruments::options::fx_option::metrics::register_fx_option_metrics(&mut registry);
    crate::instruments::options::cap_floor::metrics::register_interest_rate_option_metrics(
        &mut registry,
    );
    crate::instruments::options::credit_option::metrics::register_credit_option_metrics(
        &mut registry,
    );
    crate::instruments::options::swaption::metrics::register_swaption_metrics(&mut registry);
    crate::instruments::fixed_income::loan::metrics::register_loan_metrics(&mut registry);
    crate::instruments::fixed_income::repo::metrics::register_repo_metrics(&mut registry);
    risk::register_risk_metrics(&mut registry);
    registry
}
