//! FX Spot-specific metric calculators.
//!
//! Provides metric calculators for FX Spot instruments including spot rate,
//! base amount, quote amount, and inverse rate. These are exposed as custom
//! metrics via `MetricId::custom("...")` and registered under the
//! instrument type "FxSpot".

use crate::instruments::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;

/// Calculates the FX spot rate as quote_amount / base_amount.
pub struct SpotRateCalculator;

impl MetricCalculator for SpotRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx = match &*context.instrument {
            Instrument::FxSpot(fx) => fx,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let base_amt = fx.effective_notional().amount();
        if base_amt == 0.0 {
            return Ok(0.0);
        }
        Ok(context.base_value.amount() / base_amt)
    }
}

/// Returns the base amount (notional) in base currency units.
pub struct BaseAmountCalculator;

impl MetricCalculator for BaseAmountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx = match &*context.instrument {
            Instrument::FxSpot(fx) => fx,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };
        Ok(fx.effective_notional().amount())
    }
}

/// Returns the quote amount (PV in quote currency).
pub struct QuoteAmountCalculator;

impl MetricCalculator for QuoteAmountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        Ok(context.base_value.amount())
    }
}

/// Calculates the inverse of the spot rate (base per quote) if non-zero.
pub struct InverseRateCalculator;

impl MetricCalculator for InverseRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx = match &*context.instrument {
            Instrument::FxSpot(fx) => fx,
            _ => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };
        let base_amt = fx.effective_notional().amount();
        if base_amt == 0.0 {
            return Ok(0.0);
        }
        let spot = context.base_value.amount() / base_amt;
        if spot == 0.0 { Ok(0.0) } else { Ok(1.0 / spot) }
    }
}

/// Registers all FX Spot metrics to a registry.
pub fn register_fx_spot_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::custom("spot_rate"),
            Arc::new(SpotRateCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("base_amount"),
            Arc::new(BaseAmountCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("quote_amount"),
            Arc::new(QuoteAmountCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("inverse_rate"),
            Arc::new(InverseRateCalculator),
            &["FxSpot"],
        );
}


