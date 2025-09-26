//! Inflation01 (inflation rate sensitivity) metric for `InflationSwap`.

use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates Inflation01 (1bp inflation rate sensitivity) for inflation swaps.
pub struct Inflation01Calculator;

impl MetricCalculator for Inflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context
            .curves
            .get_discount_ref(
            s.disc_id,
        )?;
        let base = disc.base_date();

        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_index".to_string(),
                })
            })?;

        let inflation_curve =
            context
                .curves
                .get_inflation_ref(
                    s.inflation_id,
                )?;

        let i_start = inflation_index.value_on(s.start)?;

        // Align maturity CPI with discount base and apply index lag
        let lag_policy = s.lag_override.unwrap_or(inflation_index.lag());
        let lagged_maturity = match lag_policy {
            finstack_core::market_data::scalars::inflation_index::InflationLag::None => s.maturity,
            finstack_core::market_data::scalars::inflation_index::InflationLag::Months(m) => {
                finstack_core::dates::add_months(s.maturity, -(m as i32))
            }
            finstack_core::market_data::scalars::inflation_index::InflationLag::Days(d) => {
                s.maturity - time::Duration::days(d as i64)
            }
            _ => s.maturity,
        };
        let t_maturity = DayCount::Act365F
            .year_fraction(
                base,
                lagged_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity);

        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        let inflation_sensitivity =
            s.notional.amount() * (i_maturity_projected / i_start) * df * t_maturity * 0.0001;

        let signed_sensitivity = match s.side {
            PayReceiveInflation::PayFixed => inflation_sensitivity,
            PayReceiveInflation::ReceiveFixed => -inflation_sensitivity,
        };

        Ok(signed_sensitivity)
    }
}
