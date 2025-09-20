//! Core FRA pricing engine and helpers.
//!
//! Provides deterministic pricing for a Forward Rate Agreement (FRA) with
//! settlement at the start of the accrual period. The payoff is the difference
//! between the realized forward rate and the fixed rate, times the accrual
//! factor and notional, discounted to the settlement date according to the
//! discount curve's own time basis.
//!
//! PV = (F(t_start, t_end) - K) × tau(start, end) × Notional × DF(start)
//!
//! Time mappings use the instrument `day_count`. Discounting uses the
//! discount curve's date-based helper to preserve curve policy.

use crate::instruments::fra::types::ForwardRateAgreement;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Common FRA pricing engine providing core calculation methods.
pub struct FraEngine;

impl FraEngine {
    /// Calculates the present value of a FRA.
    ///
    /// # Arguments
    /// - `fra` — FRA instrument parameters
    /// - `context` — Market context containing discount and forward curves
    ///
    /// # Returns
    /// Net present value of the discounted payoff at settlement.
    pub fn pv(fra: &ForwardRateAgreement, context: &MarketContext) -> Result<Money> {
        let disc = context.get_ref::<DiscountCurve>(fra.disc_id.as_str())?;
        let fwd = context.get_ref::<ForwardCurve>(fra.forward_id.as_str())?;

        // Time fractions
        let base_date = disc.base_date();
        let _t_fixing = fra
            .day_count
            .year_fraction(base_date, fra.fixing_date, DayCountCtx::default())?
            .max(0.0);
        let t_start = fra
            .day_count
            .year_fraction(base_date, fra.start_date, DayCountCtx::default())?
            .max(0.0);
        let t_end = fra
            .day_count
            .year_fraction(base_date, fra.end_date, DayCountCtx::default())?
            .max(t_start);

        // Accrual factor
        let tau = fra
            .day_count
            .year_fraction(fra.start_date, fra.end_date, DayCountCtx::default())?
            .max(0.0);
        // If the accrual length is zero, PV is zero. When fixing is in the past,
        // continue to project using forwards unless an observed fixing is wired.
        if tau == 0.0 {
            return Ok(Money::new(0.0, fra.notional.currency()));
        }

        // Forward rate over the period and DF to settlement (start)
        let forward_rate = fwd.rate_period(t_start, t_end);
        let df_settlement = disc.df_on_date_curve(fra.start_date);

        let rate_diff = forward_rate - fra.fixed_rate;
        let pv = fra.notional.amount() * rate_diff * tau * df_settlement;
        let signed_pv = if fra.pay_fixed { -pv } else { pv };
        Ok(Money::new(signed_pv, fra.notional.currency()))
    }
}
