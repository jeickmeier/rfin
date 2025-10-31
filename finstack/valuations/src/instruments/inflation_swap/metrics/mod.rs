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
mod dv01;
mod fixed_leg_pv;
mod inflation01;
mod inflation_leg_pv;
mod par_rate;
// risk_bucketed_dv01 and theta now using generic implementations

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
            MetricId::Npv01,
            Arc::new(dv01::InflationSwapDv01Calculator),
            &["InflationSwap"],
        );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "InflationSwap",
        metrics: [
            (ParRate, par_rate::ParRateCalculator),
            (Dv01, dv01::InflationSwapDv01Calculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::InflationSwap,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::InflationSwap,
            >::default()),
        ]
    }
}
