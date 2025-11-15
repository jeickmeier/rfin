//! CMS option metrics module.
//!
//! Provides greek coverage for CMS options using finite difference methods.
//! Note: Some metrics (delta, convexity adjustment risk) require the CMS pricer
//! to be fully implemented to compute forward swap rates and convexity adjustments.

mod convexity_adjustment_risk;
mod delta;
mod rho;
mod vanna;
mod vega;
mod volga;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register CMS option metrics with the registry.
pub fn register_cms_option_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "CmsOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Vega, vega::VegaCalculator),
            (Rho, rho::RhoCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CmsOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::CmsOption,
            >::default()),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CmsOption,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    }

    // Convexity adjustment risk (custom metric)
    registry.register_metric(
        MetricId::ConvexityAdjustmentRisk,
        Arc::new(convexity_adjustment_risk::ConvexityAdjustmentRiskCalculator),
        &["CmsOption"],
    );
}
