//! Expected variance metric (blend of realized and forward).

use super::super::types::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result};

/// Calculate the expected variance (blend of realized and forward).
pub struct ExpectedVarianceCalculator;

impl MetricCalculator for ExpectedVarianceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;

        if as_of >= swap.maturity {
            // At maturity, expected variance equals realized variance; reuse partial calc on full window
            let rv = swap.partial_realized_variance(&context.curves, as_of)?;
            return Ok(rv);
        }

        if as_of < swap.start_date {
            if let Ok(scalar) = context
                .curves
                .price(format!("{}_IMPL_VOL", swap.underlying_id))
            {
                let vol = match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                };
                return Ok(vol * vol);
            }
            return Ok(swap.strike_variance);
        }

        let realized = swap.partial_realized_variance(&context.curves, as_of)?;
        let forward = if let Ok(scalar) = context
            .curves
            .price(format!("{}_IMPL_VOL", swap.underlying_id))
        {
            let vol = match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            };
            vol * vol
        } else {
            swap.strike_variance
        };

        let w = swap.realized_fraction_by_observations(as_of);
        Ok(realized * w + forward * (1.0 - w))
    }
}
