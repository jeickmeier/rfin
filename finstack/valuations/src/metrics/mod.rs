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
//! # Quick Start Examples
//!
//! ## Example 1: Computing Bucketed DV01 for a Bond
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{Bond, Instrument};
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::market_data::context::MarketContext;
//! use time::macros::date;
//!
//! # fn main() -> finstack_core::Result<()> {
//! // Setup: create an example bond and an (empty) market context.
//! // Note: real runs require a populated market context with required curves.
//! let as_of = date!(2025-01-01);
//! let bond = Bond::example();
//! let market = MarketContext::new();
//! let metrics = vec![MetricId::BucketedDv01];
//!
//! // Price with metrics
//! let result = bond.price_with_metrics(&market, as_of, &metrics)?;
//!
//! // Access results
//! let pv = result.value.amount();
//! println!("Bond PV: ${:.2}", pv);
//!
//! // Get total DV01 (scalar)
//! if let Some(total_dv01) = result.measures.get(MetricId::BucketedDv01.as_str()) {
//!     println!("Total DV01: ${:.2} per bp", total_dv01);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Computing Parallel DV01 for an Interest Rate Swap
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{Instrument, InterestRateSwap};
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::market_data::context::MarketContext;
//! use time::macros::date;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = date!(2025-01-01);
//! let swap = InterestRateSwap::example()?;
//! let market = MarketContext::new();
//! let metrics = vec![MetricId::Dv01]; // Parallel DV01
//!
//! let result = swap.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(dv01) = result.measures.get(MetricId::Dv01.as_str()) {
//!     println!("Swap DV01: ${:.2} per bp", dv01);
//!     // Negative DV01 means swap loses value when rates rise
//!     // (typical for receiver swaps)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 3: Computing Theta (Time Decay) for an Option
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{EquityOption, Instrument, PricingOverrides};
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let expiry = create_date(2024, Month::July, 1)?; // 6-month option
//!
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     expiry,
//!     finstack_core::money::Money::new(100_000.0, finstack_core::currency::Currency::USD),
//!     100.0,
//! );
//! let market = MarketContext::new();
//! let metrics = vec![MetricId::Theta];
//!
//! let result = option.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Option 1-week theta: ${:.2}", theta);
//!     // Negative theta = option loses value over time (time decay)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 4: Computing Multiple Greeks for an Option
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{EquityOption, Instrument};
//! use finstack_valuations::metrics::MetricId;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     create_date(2024, Month::July, 1)?,
//!     finstack_core::money::Money::new(100_000.0, finstack_core::currency::Currency::USD),
//!     100.0,
//! );
//! let market = MarketContext::new();
//! let metrics = vec![
//!     MetricId::Delta,
//!     MetricId::Gamma,
//!     MetricId::Vega,
//!     MetricId::Theta,
//!     MetricId::Rho,
//! ];
//!
//! let result = option.price_with_metrics(&market, as_of, &metrics)?;
//!
//! println!("Option Greeks:");
//! println!("  PV:    ${:.2}", result.value.amount());
//! println!(
//!     "  Delta: {:.4}",
//!     result.measures.get(MetricId::Delta.as_str()).unwrap_or(&0.0)
//! );
//! println!(
//!     "  Gamma: {:.4}",
//!     result.measures.get(MetricId::Gamma.as_str()).unwrap_or(&0.0)
//! );
//! println!(
//!     "  Vega:  {:.4}",
//!     result.measures.get(MetricId::Vega.as_str()).unwrap_or(&0.0)
//! );
//! println!(
//!     "  Theta: {:.4}",
//!     result.measures.get(MetricId::Theta.as_str()).unwrap_or(&0.0)
//! );
//! println!(
//!     "  Rho:   {:.4}",
//!     result.measures.get(MetricId::Rho.as_str()).unwrap_or(&0.0)
//! );
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! - **`MetricId`**: Strongly-typed identifiers for all metrics
//! - **`MetricCalculator`**: Trait for implementing custom metrics
//! - **`MetricContext`**: Context containing instrument, market data, and cached results
//! - **`MetricRegistry`**: Registry for managing calculators and dependencies
//! - **Risk metrics**: Specialized calculators for DV01, bucketed risk, and time decay
//!

// Internal submodules (organized by concern)

// calculators module removed - GenericPv was the only calculator and has been removed
mod core;
pub mod risk;
mod sensitivities;

// Re-export all public items at the root level for backward compatibility
pub use crate::instruments::common::pricing::HasDiscountCurve;
pub use core::finite_difference::{
    bump_discount_curve_parallel, bump_scalar_price, bump_sizes, scale_surface,
};
pub use core::ids::MetricId;
pub use core::registry::{MetricRegistry, StrictMode};
pub use core::traits::{MetricCalculator, MetricContext, Structured2D, Structured3D};
pub use sensitivities::cs01::{
    compute_key_rate_cs01_series_with_context, compute_parallel_cs01_with_context,
    standard_credit_cs01_buckets, GenericBucketedCs01, GenericParallelCs01, HasCreditCurve,
};
pub use sensitivities::dv01::{
    format_bucket_label, standard_ir_dv01_buckets, CurveSelection, Dv01CalculatorConfig,
    Dv01ComputationMode, ParRateContext, UnifiedDv01Calculator,
};
pub use sensitivities::fd_greeks::{
    GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVega, GenericFdVolga, HasDayCount,
    HasExpiry, HasPricingOverrides,
};
pub use sensitivities::rho::GenericRho;
pub use sensitivities::theta::{
    calculate_theta_date, generic_theta_calculator, parse_period_days, GenericTheta,
    GenericThetaAny,
};
pub use sensitivities::vega::{
    standard_equity_expiry_buckets, standard_strike_ratios, BucketSelector, KeyRateVega,
    ParallelVega,
};

// Risk metrics
pub use risk::{
    calculate_portfolio_var, calculate_var, extract_risk_factors, GenericHVar, MarketHistory,
    MarketScenario, RiskFactorShift, RiskFactorType, VarConfig, VarMethod, VarResult,
};

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Define a trivial metric calculator that delegates to an instrument method or closure.
#[macro_export]
macro_rules! define_metric_calculator {
    (
        $(#[$meta:meta])*
        $name:ident,
        instrument = $instrument:ty,
        calc = |$inst:ident, $ctx:ident| $body:expr
        $(, deps = [$($dep:expr),* $(,)?])?
    ) => {
        $(#[$meta])*
        pub struct $name;

        impl $crate::metrics::MetricCalculator for $name {
            fn calculate(
                &self,
                $ctx: &mut $crate::metrics::MetricContext,
            ) -> finstack_core::Result<f64> {
                let $inst: &$instrument = $ctx.instrument_as()?;
                let value: finstack_core::Result<f64> = { $body };
                value
            }

            fn dependencies(&self) -> &[$crate::metrics::MetricId] {
                static DEPS: &[$crate::metrics::MetricId] = &[$($($dep),*)?];
                DEPS
            }
        }
    };
}

// -----------------------------------------------------------------------------
// Error helper functions
// -----------------------------------------------------------------------------

/// Create a NotFound error for missing metrics.
///
/// Use this when a metric dependency or required metric is not available.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::metrics::{metric_not_found, MetricId};
/// use finstack_core::Result;
/// use finstack_core::collections::HashMap;
///
/// fn get_metric(id: MetricId, results: &HashMap<MetricId, f64>) -> Result<f64> {
///     results.get(&id).copied().ok_or_else(|| metric_not_found(id))
/// }
/// ```
#[inline]
pub fn metric_not_found(metric: MetricId) -> finstack_core::Error {
    finstack_core::error::InputError::NotFound {
        id: format!("metric:{metric:?}"),
    }
    .into()
}

/// Create a NotFound error for missing context fields.
///
/// Use this when a required field is not present in a context or configuration.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::metrics::context_not_found;
/// use finstack_core::types::CurveId;
/// use finstack_core::Result;
///
/// struct PricingContext {
///     discount_curve_id: Option<CurveId>,
/// }
///
/// impl PricingContext {
///     fn discount_curve_id(&self) -> Option<&CurveId> {
///         self.discount_curve_id.as_ref()
///     }
/// }
///
/// fn get_curve_id(context: &PricingContext) -> Result<&CurveId> {
///     context
///         .discount_curve_id()
///         .ok_or_else(|| context_not_found("discount_curve_id"))
/// }
/// ```
#[inline]
pub fn context_not_found(field: &str) -> finstack_core::Error {
    finstack_core::error::InputError::NotFound {
        id: format!("context.{field}"),
    }
    .into()
}
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

    // Universal metrics (work with any instrument via trait object)
    registry.register_metric(
        MetricId::Theta,
        std::sync::Arc::new(GenericThetaAny),
        &[], // Empty = applies to all instruments
    );
    registry.register_metric(
        MetricId::HVAR,
        std::sync::Arc::new(GenericHVar::var_95()),
        &[], // Empty = applies to all instruments
    );
    registry.register_metric(
        MetricId::EXPECTED_SHORTFALL,
        std::sync::Arc::new(GenericHVar::var_95()),
        &[], // Empty = applies to all instruments
    );

    // Register generic CS01 calculator for credit instruments
    // Uses GenericBucketedCs01 which computes key-rate CS01 by default
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(
            GenericBucketedCs01::<crate::instruments::CreditDefaultSwap>::default(),
        ),
        &[crate::pricer::InstrumentType::CDS],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(
            GenericBucketedCs01::<crate::instruments::cds_index::CDSIndex>::default(),
        ),
        &[crate::pricer::InstrumentType::CDSIndex],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::cds_tranche::CdsTranche,
        >::default()),
        &[crate::pricer::InstrumentType::CDSTranche],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::cds_option::CdsOption,
        >::default()),
        &[crate::pricer::InstrumentType::CDSOption],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::revolving_credit::RevolvingCredit,
        >::default()),
        &[crate::pricer::InstrumentType::RevolvingCredit],
    );

    crate::instruments::equity::metrics::register_equity_metrics(&mut registry);
    crate::instruments::basket::metrics::register_basket_metrics(&mut registry);
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    crate::instruments::fra::metrics::register_fra_metrics(&mut registry);
    crate::instruments::ir_future::metrics::register_ir_future_metrics(&mut registry);
    crate::instruments::bond_future::metrics::register_bond_future_metrics(&mut registry);
    crate::instruments::cds::metrics::register_cds_metrics(&mut registry);
    crate::instruments::cds_index::metrics::register_cds_index_metrics(&mut registry);
    crate::instruments::cds_tranche::metrics::register_cds_tranche_metrics(&mut registry);
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

    // Structured credit metrics (unified)
    crate::instruments::structured_credit::metrics::register_structured_credit_metrics(
        &mut registry,
    );
    crate::instruments::repo::metrics::register_repo_metrics(&mut registry);
    crate::instruments::term_loan::metrics::register_term_loan_metrics(&mut registry);
    crate::instruments::revolving_credit::metrics::register_revolving_credit_metrics(&mut registry);
    crate::instruments::basis_swap::metrics::register_basis_swap_metrics(&mut registry);
    crate::instruments::trs::metrics::register_trs_metrics(&mut registry);
    crate::instruments::variance_swap::metrics::register_variance_swap_metrics(&mut registry);
    crate::instruments::private_markets_fund::register_private_markets_fund_metrics(&mut registry);
    // Commodity instruments
    crate::instruments::commodity_forward::metrics::register_commodity_forward_metrics(
        &mut registry,
    );
    crate::instruments::commodity_swap::metrics::register_commodity_swap_metrics(&mut registry);
    // Exotic options
    #[cfg(feature = "mc")]
    {
        crate::instruments::asian_option::metrics::register_asian_option_metrics(&mut registry);
        crate::instruments::autocallable::metrics::register_autocallable_metrics(&mut registry);
        crate::instruments::barrier_option::metrics::register_barrier_option_metrics(&mut registry);
        crate::instruments::cliquet_option::metrics::register_cliquet_option_metrics(&mut registry);
        crate::instruments::fx_barrier_option::metrics::register_fx_barrier_option_metrics(
            &mut registry,
        );
        crate::instruments::lookback_option::metrics::register_lookback_option_metrics(
            &mut registry,
        );
        crate::instruments::quanto_option::metrics::register_quanto_option_metrics(&mut registry);
        crate::instruments::range_accrual::metrics::register_range_accrual_metrics(&mut registry);
    }
    crate::instruments::cms_option::metrics::register_cms_option_metrics(&mut registry);
    crate::instruments::dcf::metrics::register_dcf_metrics(&mut registry);
    crate::instruments::vol_index_future::metrics::register_vol_index_future_metrics(&mut registry);
    crate::instruments::vol_index_option::metrics::register_vol_index_option_metrics(&mut registry);
    registry
}
