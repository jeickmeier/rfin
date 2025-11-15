//! InflationSwap metrics module.
//!
//! Provides metric calculators specific to `InflationSwap`, split into focused
//! files. The calculators compose with the shared metrics framework and are
//! registered via `register_inflation_swap_metrics`.
//!
//! Exposed metrics:
//! - Breakeven inflation
//! - Fixed leg PV
//! - Inflation leg PV
//! - DV01 (approximate)
//! - Inflation01 (approximate)

mod breakeven;
mod fixed_leg_pv;
mod inflation01;
mod inflation_convexity;
mod inflation_leg_pv;
mod par_rate;
// risk_bucketed_dv01, dv01, and theta now using generic implementations

use crate::metrics::MetricRegistry;

/// Register all inflation swap metrics with the registry
pub fn register_inflation_swap_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics
    registry
        .register_metric(
            MetricId::custom("breakeven"),
            Arc::new(breakeven::BreakevenCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("fixed_leg_pv"),
            Arc::new(fixed_leg_pv::FixedLegPvCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("inflation_leg_pv"),
            Arc::new(inflation_leg_pv::InflationLegPvCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::Inflation01,
            Arc::new(inflation01::Inflation01Calculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::InflationConvexity,
            Arc::new(inflation_convexity::InflationConvexityCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::Npv01,
            Arc::new(crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InflationSwap,
            >::new(
                crate::metrics::Dv01CalculatorConfig::parallel_combined()
            )),
            &["InflationSwap"],
        );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "InflationSwap",
        metrics: [
            (ParRate, par_rate::ParRateCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InflationSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::InflationSwap,
            >::default()),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InflationSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    }
}
