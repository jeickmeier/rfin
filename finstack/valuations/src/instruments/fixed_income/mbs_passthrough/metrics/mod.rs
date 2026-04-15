//! Agency MBS risk metrics.
//!
//! This module provides MBS-specific risk metrics including:
//!
//! - **OAS (Option-Adjusted Spread)**: Spread over risk-free that equates
//!   model price to market price
//! - **Effective Duration**: Duration accounting for prepayment sensitivity
//! - **Effective Convexity**: Convexity accounting for prepayment sensitivity
//! - **Bucketed DV01**: Key-rate bucketed interest-rate sensitivities via
//!   the generic `UnifiedDv01Calculator` with triangular key-rate config.

pub(crate) mod duration;
#[allow(dead_code)] // Public API items used by external bindings
pub(crate) mod mc_oas;
pub(crate) mod oas;

pub(crate) use duration::{effective_convexity, effective_duration};
#[allow(unused_imports)] // Public API re-exports for external consumers
pub(crate) use mc_oas::{calculate_mc_oas, McOasConfig, McOasResult};
pub(crate) use oas::calculate_oas;

use crate::instruments::fixed_income::mbs_passthrough::AgencyMbsPassthrough;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};

/// Calculator for effective duration (mapped to DurationMod).
pub(crate) struct EffectiveDurationCalculator;

impl MetricCalculator for EffectiveDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        effective_duration(mbs, context.curves.as_ref(), context.as_of, None)
    }
}

/// Calculator for effective convexity (mapped to Convexity).
pub(crate) struct EffectiveConvexityCalculator;

impl MetricCalculator for EffectiveConvexityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        effective_convexity(mbs, context.curves.as_ref(), context.as_of, None)
    }
}

/// Calculator for option-adjusted spread (OAS).
pub(crate) struct OasCalculator;

impl MetricCalculator for OasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let mbs: &AgencyMbsPassthrough = context.instrument_as()?;
        let market_price = mbs
            .pricing_overrides
            .market_quotes
            .quoted_clean_price
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::InputError::NotFound {
                    id: "mbs.pricing_overrides.quoted_clean_price".to_string(),
                })
            })?;
        let result = calculate_oas(mbs, market_price, context.curves.as_ref(), context.as_of)?;
        Ok(result.oas)
    }
}

/// Register agency MBS passthrough metrics with the registry.
pub(crate) fn register_mbs_passthrough_metrics(registry: &mut MetricRegistry) {
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
        ]
    }
}
