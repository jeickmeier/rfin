//! FX01 calculator for FX Forwards.
//!
//! Computes sensitivity to a 1bp absolute bump in the spot FX rate.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_forward::FxForward;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// FX01 calculator for FX Forwards.
pub struct Fx01Calculator;

impl MetricCalculator for Fx01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fwd: &FxForward = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let base_pv = fwd.value(&curves, as_of)?;

        let domestic_disc = curves.get_discount_ref(fwd.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(fwd.foreign_discount_curve_id.as_str())?;

        let df_domestic = domestic_disc.try_df_between_dates(as_of, fwd.maturity_date)?;
        let df_foreign = foreign_disc.try_df_between_dates(as_of, fwd.maturity_date)?;

        let spot = if let Some(rate) = fwd.spot_rate_override {
            rate
        } else if let Some(fx) = curves.fx.as_ref() {
            (**fx)
                .rate(FxQuery::new(fwd.base_currency, fwd.quote_currency, as_of))?
                .rate
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        let bump = 0.0001;
        let bumped_spot = spot + bump;

        let market_forward = bumped_spot * df_foreign / df_domestic;
        let contract_fwd = fwd.contract_rate.unwrap_or(market_forward);

        let n_base = fwd.notional.amount();
        let bumped_pv = n_base * (market_forward - contract_fwd) * df_domestic;

        Ok(bumped_pv - base_pv.amount())
    }
}
