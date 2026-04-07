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
pub(crate) fn register_cms_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::CmsOption,
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Vega, vega::VegaCalculator),
            (Rho, rho::RhoCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CmsOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CmsOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }

    // Convexity adjustment risk (custom metric)
    registry.register_metric(
        MetricId::ConvexityAdjustmentRisk,
        Arc::new(convexity_adjustment_risk::ConvexityAdjustmentRiskCalculator),
        &[InstrumentType::CmsOption],
    );
}
