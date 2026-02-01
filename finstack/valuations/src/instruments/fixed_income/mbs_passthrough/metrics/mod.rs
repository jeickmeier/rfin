//! Agency MBS risk metrics.
//!
//! This module provides MBS-specific risk metrics including:
//!
//! - **OAS (Option-Adjusted Spread)**: Spread over risk-free that equates
//!   model price to market price
//! - **Effective Duration**: Duration accounting for prepayment sensitivity
//! - **Effective Convexity**: Convexity accounting for prepayment sensitivity
//! - **Key-Rate DV01**: Bucketed interest rate sensitivities

pub mod duration;
pub mod key_rate;
pub mod oas;

pub use duration::{effective_convexity, effective_duration};
pub use oas::calculate_oas;

use crate::instruments::fixed_income::mbs_passthrough::AgencyMbsPassthrough;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};

/// Calculator for effective duration (mapped to DurationMod).
pub struct EffectiveDurationCalculator;

impl MetricCalculator for EffectiveDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        effective_duration(mbs, context.curves.as_ref(), context.as_of, None)
    }
}

/// Calculator for effective convexity (mapped to Convexity).
pub struct EffectiveConvexityCalculator;

impl MetricCalculator for EffectiveConvexityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        effective_convexity(mbs, context.curves.as_ref(), context.as_of, None)
    }
}

/// Calculator for option-adjusted spread (OAS).
pub struct OasCalculator;

impl MetricCalculator for OasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        let market_price = mbs.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "mbs.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;
        let result = calculate_oas(mbs, market_price, context.curves.as_ref(), context.as_of)?;
        Ok(result.oas)
    }
}

/// Register agency MBS passthrough metrics with the registry.
pub fn register_mbs_passthrough_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::AgencyMbsPassthrough,
        metrics: [
            (DurationMod, EffectiveDurationCalculator),
            (Convexity, EffectiveConvexityCalculator),
            (Oas, OasCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::AgencyMbsPassthrough,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::AgencyMbsPassthrough,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::AgencyMbsPassthrough,
            >::default()),
        ]
    }
}
