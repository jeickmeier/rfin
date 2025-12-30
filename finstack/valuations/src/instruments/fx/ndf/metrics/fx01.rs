//! FX01 calculator for NDFs.
//!
//! Computes sensitivity to a 1bp absolute bump in the spot FX rate.

use crate::instruments::common::traits::Instrument;
use crate::instruments::ndf::Ndf;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::money::fx::FxQuery;

/// FX01 calculator for NDFs.
pub struct Fx01Calculator;

impl MetricCalculator for Fx01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let ndf: &Ndf = context.instrument_as()?;
        let curves = context.curves.clone();
        let as_of = context.as_of;

        // Post-fixing NDFs are not sensitive to spot.
        if ndf.fixing_rate.is_some() || as_of >= ndf.fixing_date {
            return Ok(0.0);
        }

        let base_pv = ndf.value(&curves, as_of)?;

        let settlement_disc = curves.get_discount_ref(ndf.settlement_curve_id.as_str())?;
        let df_settlement = settlement_disc.df_between_dates(as_of, ndf.maturity_date)?;

        // Resolve spot with override if present
        let spot = if let Some(rate) = ndf.spot_rate_override {
            rate
        } else if let Some(fx) = curves.fx() {
            match (**fx).rate(FxQuery::new(
                ndf.base_currency,
                ndf.settlement_currency,
                as_of,
            )) {
                Ok(rate) => rate.rate,
                Err(_) => {
                    let inverse = (**fx).rate(FxQuery::new(
                        ndf.settlement_currency,
                        ndf.base_currency,
                        as_of,
                    ))?;
                    1.0 / inverse.rate
                }
            }
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        let bump = 0.0001;
        let bumped_spot = spot + bump;

        // If foreign curve available, use CIRP for bumped forward
        let effective_forward = if let Some(ref foreign_curve_id) = ndf.foreign_curve_id {
            if let Ok(foreign_disc) = curves.get_discount_ref(foreign_curve_id.as_str()) {
                let df_foreign = foreign_disc.df_between_dates(as_of, ndf.maturity_date)?;
                bumped_spot * df_foreign / df_settlement
            } else {
                bumped_spot
            }
        } else {
            bumped_spot
        };

        let n_base = ndf.notional.amount();
        let settlement_amount = n_base * (1.0 / ndf.contract_rate - 1.0 / effective_forward);
        let bumped_pv = settlement_amount * df_settlement;

        Ok(bumped_pv - base_pv.amount())
    }
}
