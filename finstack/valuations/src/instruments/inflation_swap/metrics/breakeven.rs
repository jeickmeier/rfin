//! Breakeven inflation metric for `InflationSwap`.

use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates breakeven inflation rate for inflation swaps.
///
/// Computes the fixed rate that makes the swap's present value zero.
/// Formula: K_BE = (E[I(T_mat)]/I(T_start))^(1/τ) - 1
pub struct BreakevenCalculator;

impl MetricCalculator for BreakevenCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;

        let inflation_index = context
            .curves
            .inflation_index_ref(s.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_index".to_string(),
                })
            })?;

        let inflation_curve = context.curves.get_inflation_ref(s.inflation_id)?;

        let i_start = inflation_index.value_on(s.start)?;

        // Align projection time with discount curve base and apply index lag to maturity
        let disc = context.curves.get_discount_ref(s.disc_id.as_str())?;
        let base = disc.base_date();

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

        let tau_accrual = s.dc.year_fraction(
            s.start,
            s.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if i_start <= 0.0 || tau_accrual <= 0.0 {
            return Ok(0.0);
        }

        let inflation_ratio = i_maturity_projected / i_start;
        let breakeven = inflation_ratio.powf(1.0 / tau_accrual) - 1.0;

        Ok(breakeven)
    }
}
