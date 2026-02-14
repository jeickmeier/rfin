//! Implied volatility calculator for interest rate options.
//!
//! Uses root-finding to solve for the Black volatility that reproduces the
//! observed market price of the option.
//!
//! # Limitations
//!
//! This calculator is designed for **single-period caplets/floorlets only**.
//! For multi-period caps/floors, the implied volatility would require solving
//! across all caplet contributions (cap stripping), which is not yet implemented.
//! When applied to a multi-period cap/floor, this calculator uses only the first
//! period's forward rate, which may not reflect the true flat volatility.

use crate::instruments::common_impl::pricing::time::{
    rate_period_on_dates, relative_df_discount_curve,
};
use crate::instruments::rates::cap_floor::pricing::black::{
    price_caplet_floorlet, CapletFloorletInputs,
};
use crate::instruments::rates::cap_floor::{InterestRateOption, RateOptionType};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

/// Implied volatility calculator using Black model.
///
/// # Note
///
/// This metric is only accurate for single-period caplets/floorlets.
/// For multi-period caps/floors, consider using cap stripping or
/// bootstrapping techniques to extract per-caplet implied volatilities.
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;
        let strike = option.strike_rate_f64()?;

        // Need market price to solve for implied volatility.
        // The quoted_clean_price is passed via the MetricContext pricing overrides,
        // not stored on the instrument itself.
        let market_price = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.quoted_clean_price)
            .ok_or_else(|| {
                finstack_core::Error::Input(finstack_core::InputError::NotFound {
                    id: "Market price required for implied vol (set via pricing overrides)"
                        .to_string(),
                })
            })?;

        // Get curves from market context
        let forward_curve = context.curves.get_forward(&option.forward_curve_id)?;
        let discount_curve = context.curves.get_discount(&option.discount_curve_id)?;
        let dc_ctx = finstack_core::dates::DayCountCtx::default();

        // Use instrument day count for time-to-fixing (consistent with pricing and vol surface lookup)
        let time_to_fixing = if option.start_date > context.as_of {
            option
                .day_count
                .year_fraction(context.as_of, option.start_date, dc_ctx)?
        } else {
            0.0
        };

        if time_to_fixing <= 0.0 {
            return Ok(0.0); // Expired option has no vol
        }

        // Use curve-consistent helpers for forward rate and discount factor
        // (same as in the main pricing implementation)
        let forward_rate =
            rate_period_on_dates(forward_curve.as_ref(), option.start_date, option.maturity)?;
        let discount_factor =
            relative_df_discount_curve(discount_curve.as_ref(), context.as_of, option.maturity)?;

        let accrual_fraction = option.day_count.year_fraction(
            option.start_date,
            option.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let is_cap = matches!(
            option.rate_option_type,
            RateOptionType::Cap | RateOptionType::Caplet
        );

        // Set up inputs for Black model
        let base_inputs = CapletFloorletInputs {
            is_cap,
            notional: option.notional.amount(),
            strike,
            forward: forward_rate,
            discount_factor,
            volatility: 0.0, // Will be varied in solver
            time_to_fixing,
            accrual_year_fraction: accrual_fraction,
            currency: option.notional.currency(),
        };

        // Objective function: Black price - market price = 0
        let objective = |vol: f64| {
            // Keep the objective well-defined and sign-consistent so the solver can
            // bracket a root robustly.
            //
            // At vol -> 0, the Black price approaches 0, so the residual approaches
            // `-market_price`.
            if vol <= 0.0 {
                return -market_price;
            }

            let mut inputs = base_inputs;
            inputs.volatility = vol;

            match price_caplet_floorlet(inputs) {
                Ok(price) => price.amount() - market_price,
                Err(_) => market_price, // Treat pricing failure as a large positive residual
            }
        };

        // Solve for implied volatility using Brent solver
        let mut solver = BrentSolver::new().tolerance(1e-6);
        solver.max_iterations = 50;

        // Initial guess: 20% volatility, reasonable bounds 0.1% to 300%
        let implied_vol = solver.solve(objective, 0.20)?;

        // Sanity check result
        if implied_vol > 0.0 && implied_vol < 5.0 {
            Ok(implied_vol)
        } else {
            Err(finstack_core::Error::Validation(
                "Unreasonable implied volatility".to_string(),
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies on other metrics
    }
}
