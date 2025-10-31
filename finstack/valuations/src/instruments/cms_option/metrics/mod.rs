//! CMS option metrics module.
//!
//! Provides greek coverage for CMS options using finite difference methods.
//! Note: Some metrics (delta, convexity adjustment risk) require the CMS pricer
//! to be fully implemented to compute forward swap rates and convexity adjustments.

mod convexity_adjustment_risk;
mod delta;
mod dv01;
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
            (Dv01, dv01::Dv01Calculator),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::CmsOption,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CmsOption,
            >::default()),
        ]
    }

    // Convexity adjustment risk (custom metric)
    registry.register_metric(
        MetricId::custom("convexity_adjustment_risk"),
        Arc::new(convexity_adjustment_risk::ConvexityAdjustmentRiskCalculator),
        &["CmsOption"],
    );
}
