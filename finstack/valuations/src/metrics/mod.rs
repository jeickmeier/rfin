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
//! ```ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::{CurveId, Rate, Currency};
//! use finstack_core::money::Money;
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::dates::day_count::DayCount;
//!
//! # fn main() -> finstack_core::Result<()> {
//! // Setup: Create a 5-year bond
//! let as_of = create_date(2024, Month::January, 1)?;
//! let maturity = create_date(2029, Month::January, 1)?;
//! let bond = Bond::builder("BOND-001")
//!     .issue_date(as_of)
//!     .maturity(maturity)
//!     .coupon_rate(Rate::from_bps(500)) // 5.00% coupon
//!     .face_value(Money::new(100_000.0, Currency::USD))
//!     .build()?;
//!
//! // Create discount curve
//! let curve_id = CurveId::from("USD-OIS");
//! let discount_curve = DiscountCurve::builder(curve_id.clone())
//!     .base_date(as_of)
//!     .day_count(DayCount::Act365F)
//!     .knots(vec![
//!         (0.0, 1.0),
//!         (1.0, 0.96),
//!         (2.0, 0.93),
//!         (5.0, 0.85),
//!         (10.0, 0.70),
//!     ])
//!     .build()?;
//!
//! let market = MarketContext::new(as_of)
//!     .insert_discount(discount_curve);
//!
//! // Create registry and request bucketed DV01
//! let registry = standard_registry();
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
//! if let Some(total_dv01) = result.measures.get(&MetricId::BucketedDv01) {
//!     println!("Total DV01: ${:.2} per bp", total_dv01);
//! }
//!
//! // Access bucketed series (key-rate DV01 by maturity)
//! if let Some(bucketed) = result.bucketed_series.get(&MetricId::BucketedDv01) {
//!     println!("\nBucketed DV01 breakdown:");
//!     for (bucket, dv01) in bucketed {
//!         println!("  {} bucket: ${:.2} per bp", bucket, dv01);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Computing Parallel DV01 for an Interest Rate Swap
//!
//! ```ignore
//! use finstack_valuations::instruments::InterestRateSwap;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::{CurveId, Rate, Currency};
//! use finstack_core::money::Money;
//! use finstack_core::market_data::MarketContext;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//!
//! // Create a 5-year receiver swap (receive fixed, pay floating)
//! let swap = InterestRateSwap::builder("SWAP-001")
//!     .start_date(as_of)
//!     .maturity(create_date(2029, Month::January, 1)?)
//!     .notional(Money::new(10_000_000.0, Currency::USD))
//!     .fixed_rate(Rate::from_bps(300)) // 3.00% fixed
//!     .is_receive_fixed(true)
//!     .build()?;
//!
//! // Setup market with discount and forward curves
//! // (market setup omitted for brevity)
//! # let market = MarketContext::new(as_of);
//!
//! let registry = standard_registry();
//! let metrics = vec![MetricId::Dv01]; // Parallel DV01
//!
//! let result = swap.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(dv01) = result.measures.get(&MetricId::Dv01) {
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
//! ```ignore
//! use finstack_valuations::instruments::{EquityOption, PricingOverrides};
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::Currency;
//! use finstack_core::money::Money;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let expiry = create_date(2024, Month::July, 1)?; // 6-month option
//!
//! let option = EquityOption::builder("OPT-001")
//!     .strike(Money::new(100.0, Currency::USD))
//!     .expiry(expiry)
//!     .is_call(true)
//!     .build()?;
//!
//! // Setup market (omitted for brevity)
//! # use finstack_core::market_data::MarketContext;
//! # let market = MarketContext::new(as_of);
//!
//! let registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Customize theta period (default is "1D" = 1 day)
//! let mut overrides = PricingOverrides::default();
//! overrides.theta_period = Some("1W".to_string()); // 1-week time decay
//!
//! let result = option.price_with_metrics_and_overrides(
//!     &market, as_of, &metrics, &overrides
//! )?;
//!
//! if let Some(theta) = result.measures.get(&MetricId::Theta) {
//!     println!("Option 1-week theta: ${:.2}", theta);
//!     // Negative theta = option loses value over time (time decay)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 4: Computing Multiple Greeks for an Option
//!
//! ```ignore
//! use finstack_valuations::instruments::EquityOption;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::Currency;
//! use finstack_core::money::Money;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let option = EquityOption::builder("OPT-001")
//!     .strike(Money::new(100.0, Currency::USD))
//!     .expiry(create_date(2024, Month::July, 1)?)
//!     .is_call(true)
//!     .build()?;
//!
//! // Setup market
//! # use finstack_core::market_data::MarketContext;
//! # let market = MarketContext::new(as_of);
//!
//! let registry = standard_registry();
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
//! println!("  Delta: {:.4}", result.measures.get(&MetricId::Delta).unwrap_or(&0.0));
//! println!("  Gamma: {:.4}", result.measures.get(&MetricId::Gamma).unwrap_or(&0.0));
//! println!("  Vega:  {:.4}", result.measures.get(&MetricId::Vega).unwrap_or(&0.0));
//! println!("  Theta: {:.4}", result.measures.get(&MetricId::Theta).unwrap_or(&0.0));
//! println!("  Rho:   {:.4}", result.measures.get(&MetricId::Rho).unwrap_or(&0.0));
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
//! # Documentation
//!
//! Comprehensive documentation on all metrics, including formulas, conventions, and units,
//! is available in `METRICS.md` in this directory.

// Internal submodules (organized by concern)

// calculators module removed - GenericPv was the only calculator and has been removed
mod core;
mod sensitivities;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;

// Re-export all public items at the root level for backward compatibility
pub use core::finite_difference::bump_sizes;
pub use core::has_equity_underlying::HasEquityUnderlying;
pub use core::has_pricing_overrides::HasPricingOverrides;
pub use core::ids::MetricId;
pub use core::registry::MetricRegistry;
pub use core::traits::{MetricCalculator, MetricContext, Structured2D, Structured3D};
pub use sensitivities::cs01::{
    compute_key_rate_cs01_series, compute_key_rate_cs01_series_with_context, compute_parallel_cs01,
    compute_parallel_cs01_with_context, standard_credit_cs01_buckets, GenericBucketedCs01,
    GenericParallelCs01, HasCreditCurve,
};
pub use sensitivities::dv01::{
    compute_key_rate_dv01_series, compute_key_rate_dv01_series_with_context,
    compute_key_rate_series_for_id, compute_key_rate_series_with_context_for_id,
    compute_parallel_dv01, compute_parallel_dv01_with_context, standard_ir_dv01_buckets,
    GenericBucketedDv01, GenericBucketedDv01WithContext, GenericParallelDv01, HasDiscountCurve,
    HasForwardCurves, ParallelDv01Mode,
};
pub use sensitivities::fd_greeks::{
    GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVega, GenericFdVolga,
};
pub use sensitivities::shock_mode::{BucketSelector, ShockMode};
pub use sensitivities::theta::{calculate_theta_date, parse_period_days, GenericTheta, GenericThetaAny};
pub use sensitivities::utils::dv01_from_modified_duration;
pub use sensitivities::vega::{
    compute_bucketed_vega_matrix, compute_parallel_vega, standard_equity_expiry_buckets,
    standard_strike_ratios, GenericVega, VOL_BUMP_PCT,
};
pub use sensitivities::vol::{
    get_instrument_day_count, get_instrument_expiry_for_adaptive, get_instrument_vol_id,
};

// Compatibility shims for legacy module paths
// These allow existing code using `crate::metrics::bucketed_dv01::*` to continue working

/// Legacy module for bucketed DV01. Use `sensitivities::dv01` internally.
pub mod bucketed_dv01 {
    pub use super::sensitivities::dv01::*;
}

/// Legacy module for bucketed CS01. Use `sensitivities::cs01` internally.
pub mod bucketed_cs01 {
    pub use super::sensitivities::cs01::*;
}

/// Legacy module for bucketed vega. Use `sensitivities::vega` internally.
pub mod bucketed_vega {
    pub use super::sensitivities::vega::*;
}

/// Legacy module for finite difference Greeks. Use `sensitivities::fd_greeks` internally.
pub mod fd_greeks {
    pub use super::sensitivities::fd_greeks::*;
}

/// Legacy module for finite difference utilities. Use `core::finite_difference` internally.
pub mod finite_difference {
    pub use super::core::finite_difference::*;
}

/// Legacy module for volatility helpers. Use `sensitivities::vol` internally.
pub mod vol_expiry_helpers {
    pub use super::sensitivities::vol::*;
}

/// Legacy module for equity underlying trait. Use `core::has_equity_underlying` internally.
pub mod has_equity_underlying {
    pub use super::core::has_equity_underlying::*;
}

/// Legacy module for pricing overrides trait. Use `core::has_pricing_overrides` internally.
pub mod has_pricing_overrides {
    pub use super::core::has_pricing_overrides::*;
}

/// Legacy module for theta utilities. Use `sensitivities::theta` internally.
pub mod theta_utils {
    pub use super::sensitivities::theta::*;
}

/// Legacy module for helper utilities. Use `sensitivities::utils` internally.
pub mod helpers {
    pub use super::sensitivities::utils::*;
}

/// Legacy module for shock mode. Use `sensitivities::shock_mode` internally.
pub mod shock_mode {
    pub use super::sensitivities::shock_mode::*;
}

/// Legacy module for metric IDs. Use `core::ids` internally.
pub mod ids {
    pub use super::core::ids::*;
}

/// Legacy module for metric registry. Use `core::registry` internally.
pub mod registry {
    pub use super::core::registry::*;
}

/// Legacy module for metric traits. Use `core::traits` internally.
pub mod traits {
    pub use super::core::traits::*;
}

/// Legacy module for registration macro. Use `core::registration_macro` internally.
#[allow(unused_imports)]
pub mod registration_macro {
    pub use super::core::registration_macro::*;
}

// Legacy PV module removed - PV is always available in ValuationResult.val

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

    // Register universal Theta calculator for ALL instruments (empty applicability = all)
    registry.register_metric(
        MetricId::Theta,
        std::sync::Arc::new(GenericThetaAny),
        &[], // Empty = applies to all instruments
    );

    // Register generic CS01 calculator for credit instruments
    // Uses GenericBucketedCs01 which computes key-rate CS01 by default
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(
            GenericBucketedCs01::<crate::instruments::CreditDefaultSwap>::default(),
        ),
        &["CreditDefaultSwap"],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(
            GenericBucketedCs01::<crate::instruments::cds_index::CDSIndex>::default(),
        ),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::cds_tranche::CdsTranche,
        >::default()),
        &["CdsTranche"],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::cds_option::CdsOption,
        >::default()),
        &["CdsOption"],
    );
    registry.register_metric(
        MetricId::BucketedCs01,
        std::sync::Arc::new(GenericBucketedCs01::<
            crate::instruments::revolving_credit::RevolvingCredit,
        >::default()),
        &["RevolvingCredit"],
    );

    // TODO: Add CS01 for Bond once hazard-rate pricing is implemented
    // TODO: Add CS01 for StructuredCredit once hazard-rate pricing is added
    // TODO: Add CS01 for Convertible when priced with credit risk (implement HasCreditCurve)

    crate::instruments::equity::metrics::register_equity_metrics(&mut registry);
    crate::instruments::basket::metrics::register_basket_metrics(&mut registry);
    crate::instruments::bond::metrics::register_bond_metrics(&mut registry);
    crate::instruments::irs::metrics::register_irs_metrics(&mut registry);
    crate::instruments::deposit::metrics::register_deposit_metrics(&mut registry);
    crate::instruments::fra::metrics::register_fra_metrics(&mut registry);
    crate::instruments::ir_future::metrics::register_ir_future_metrics(&mut registry);
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
    registry
}
