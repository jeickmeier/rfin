//! Inflation swap pricing engine.
//!
//! Implements leg PV calculations for a zero-coupon inflation swap using
//! core market objects:
//! - Discounting via `DiscountCurve`
//! - Inflation projection via `InflationCurve`
//! - Historical index via `InflationIndex`
//!
//! The formulas align with the instrument's `types` helpers but are
//! encapsulated here to follow the pricing separation standard seen in CDS.

use crate::instruments::inflation_swap::types::InflationSwap;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;

/// Inflation swap pricing engine.
#[derive(Default, Clone, Debug)]
pub struct InflationSwapPricer;

impl InflationSwapPricer {
    /// Create a new inflation swap pricer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Present value of the fixed (real) leg.
    pub fn pv_fixed_leg(
        &self,
        s: &InflationSwap,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.disc_id,
            )?;
        let base = disc.base_date();

        let tau_accrual = s.dc.year_fraction(
            s.start,
            s.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let fixed_payment = s.notional * ((1.0 + s.fixed_rate).powf(tau_accrual) - 1.0);

        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(fixed_payment * df)
    }

    /// Present value of the inflation leg.
    pub fn pv_inflation_leg(
        &self,
        s: &InflationSwap,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.disc_id,
            )?;
        let base = disc.base_date();

        let inflation_index = curves
            .inflation_index_ref(s.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound {
                        id: "inflation_index".to_string(),
                    },
                )
            })?;

        let inflation_curve = curves.get_ref::<
            finstack_core::market_data::term_structures::inflation::InflationCurve,
        >(s.inflation_id)?;

        let i_start = inflation_index.value_on(s.start)?;

        // Apply the same lag policy as the index to the maturity for projection
        // Use contract override if present, else index lag
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

        // Use a common base (discount base) to derive time in years for the inflation curve
        let t_maturity_infl = DayCount::Act365F
            .year_fraction(
                base,
                lagged_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity_infl);

        let inflation_payment = s.notional * (i_maturity_projected / i_start - 1.0);

        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(inflation_payment * df)
    }
}


