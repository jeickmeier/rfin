//! Core IR Future pricing engine.
//!
//! Provides deterministic valuation for exchange-traded interest rate futures.
//! The PV represents the value per contract given the difference between the
//! model forward rate (with convexity adjustment) and the implied rate from
//! the quoted future price, multiplied by the contract face value and accrual
//! fraction over the underlying rate period.
//!
//! PV = (R_model_adj − R_implied) × FaceValue × tau(period_start, period_end)
//!
//! Time mappings use the instrument's `day_count` for both the underlying
//! period and the mapping from curve base date to the underlying period.

use crate::instruments::ir_future::{InterestRateFuture, Position};
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Pricing engine for `InterestRateFuture`.
pub struct IrFutureEngine;

impl IrFutureEngine {
    /// Calculates the present value of an interest rate future.
    ///
    /// Delegates to the instrument's rate and convexity policy and uses
    /// discount/forward curves from the `MarketContext`.
    pub fn pv(fut: &InterestRateFuture, context: &MarketContext) -> Result<Money> {
        let disc = context.get_ref::<DiscountCurve>(fut.disc_id.clone())?;
        let fwd = context.get_ref::<ForwardCurve>(fut.forward_id.clone())?;

        // Base date for mapping to curve time
        let base_date = disc.base_date();

        // Time to fixing and rate period on the instrument basis
        let t_fixing = fut
            .day_count
            .year_fraction(base_date, fut.fixing_date, DayCountCtx::default())?
            .max(0.0);
        let t_start = fut
            .day_count
            .year_fraction(base_date, fut.period_start, DayCountCtx::default())?
            .max(0.0);
        let t_end = fut
            .day_count
            .year_fraction(base_date, fut.period_end, DayCountCtx::default())?
            .max(t_start);

        // Forward rate over the period
        let forward_rate = fwd.rate_period(t_start, t_end);

        // Apply convexity adjustment policy
        let adjusted_rate = if let Some(ca) = fut.contract_specs.convexity_adjustment {
            forward_rate + ca
        } else {
            // Estimate convexity using a Hull-White style approximation
            let vol_estimate = if t_fixing <= 0.25 {
                0.008
            } else if t_fixing <= 0.5 {
                0.0085
            } else if t_fixing <= 1.0 {
                0.009
            } else if t_fixing <= 2.0 {
                0.0095
            } else {
                0.01
            };
            let tau_len = t_end - t_start;
            let convexity = 0.5 * vol_estimate * vol_estimate * t_fixing * (t_fixing + tau_len);
            forward_rate + convexity
        };

        // Implied rate from price and accrual over the underlying period
        let implied_rate = fut.implied_rate();
        let tau = fut
            .day_count
            .year_fraction(fut.period_start, fut.period_end, DayCountCtx::default())?
            .max(0.0);
        if tau == 0.0 {
            return Ok(Money::new(0.0, fut.notional.currency()));
        }

        // Position sign: Long benefits when implied > model (rates down → price up)
        let sign = match fut.position {
            Position::Long => 1.0,
            Position::Short => -1.0,
        };

        // Scale by contracts: notional may represent multiples of face value
        let contracts_scale = if fut.contract_specs.face_value != 0.0 {
            fut.notional.amount() / fut.contract_specs.face_value
        } else {
            1.0
        };

        let pv_per_contract = (implied_rate - adjusted_rate) * fut.contract_specs.face_value * tau;
        let pv_total = sign * contracts_scale * pv_per_contract;
        Ok(Money::new(pv_total, fut.notional.currency()))
    }
}
