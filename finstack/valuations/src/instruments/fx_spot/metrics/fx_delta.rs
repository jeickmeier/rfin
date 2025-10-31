//! FX Delta calculator for FX Spot.
//!
//! Computes FX delta (FX spot sensitivity) using finite differences.
//! FX delta measures the change in PV for a 1% move in the FX spot rate.
//!
//! For FxSpot, this is straightforward: if we hold base currency, a 1% increase
//! in spot means we receive 1% more in quote currency.

use crate::instruments::common::metrics::finite_difference::bump_sizes;
use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;
use finstack_core::Result;

/// FX Delta calculator for FX Spot.
pub struct FxDeltaCalculator;

impl MetricCalculator for FxDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_spot: &FxSpot = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Get current FX spot rate
        let fx_matrix = context.curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        let current_rate = (**fx_matrix)
            .rate(FxQuery::new(fx_spot.base, fx_spot.quote, as_of))?
            .rate;

        // Bump spot rate by 1%
        let rate_bump = current_rate * bump_sizes::SPOT;
        let bumped_rate = current_rate + rate_bump;

        // For simplicity, we'll create a new instrument with bumped rate
        // and reprice, since FxSpot can use either market rate or explicit rate
        let mut fx_spot_bumped = fx_spot.clone();
        fx_spot_bumped.spot_rate = Some(bumped_rate);
        let pv_bumped = fx_spot_bumped.npv(context.curves.as_ref(), as_of)?.amount();

        // FX Delta = (PV_bumped - PV_base) / bump_size
        // Since bump is 1% of rate, we divide by bump_size to get per 1% move
        let fx_delta = (pv_bumped - base_pv) / bump_sizes::SPOT;

        Ok(fx_delta)
    }
}

