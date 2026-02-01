//! Agency CMO risk metrics.
//!
//! CMO-specific metrics including tranche-level OAS, duration,
//! and scenario analysis.

pub mod oas;

pub use oas::calculate_tranche_oas;

use crate::instruments::fixed_income::cmo::AgencyCmo;
use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};

/// Calculator for tranche option-adjusted spread (OAS).
pub struct OasCalculator;

impl MetricCalculator for OasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmo: &AgencyCmo = context.instrument_as()?;
        let market_price = cmo.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "cmo.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;
        let result =
            calculate_tranche_oas(cmo, market_price, context.curves.as_ref(), context.as_of)?;
        Ok(result.oas)
    }
}

/// Register agency CMO metrics with the registry.
pub fn register_cmo_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::AgencyCmo,
        metrics: [
            (Oas, OasCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::AgencyCmo,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::AgencyCmo,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::AgencyCmo,
            >::default()),
        ]
    }
}
