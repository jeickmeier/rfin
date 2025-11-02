//! Implied volatility calculator for interest rate options.
//!
//! Uses root-finding to solve for the Black volatility that reproduces the
//! observed market price of the option.

use crate::instruments::cap_floor::pricing::black::{price_caplet_floorlet, CapletFloorletInputs};
use crate::instruments::cap_floor::{InterestRateOption, RateOptionType};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

/// Implied volatility calculator using Black model
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Need market price to solve for implied volatility
        let market_price = option.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "Market price required for implied vol".to_string(),
            })
        })?;

        // Get forward rate from market context
        let forward_curve = context.curves.get_forward_ref(&option.forward_id)?;
        let discount_curve = context.curves.get_discount_ref(&option.discount_curve_id)?;

        // For single period caplet/floorlet, use simple calculation
        let time_to_fixing = if option.start_date > context.as_of {
            forward_curve.day_count().year_fraction(
                context.as_of,
                option.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
        } else {
            0.0
        };

        if time_to_fixing <= 0.0 {
            return Ok(0.0); // Expired option has no vol
        }

        let forward_rate = forward_curve.rate(time_to_fixing);
        let time_to_payment = forward_curve.day_count().year_fraction(
            context.as_of,
            option.end_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let discount_factor = discount_curve.df(time_to_payment);

        let accrual_fraction = option.day_count.year_fraction(
            option.start_date,
            option.end_date,
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
            strike: option.strike_rate,
            forward: forward_rate,
            discount_factor,
            volatility: 0.0, // Will be varied in solver
            time_to_fixing,
            accrual_year_fraction: accrual_fraction,
            currency: option.notional.currency(),
        };

        // Objective function: Black price - market price = 0
        let objective = |vol: f64| {
            if vol <= 0.0 {
                return market_price; // High error for negative vols
            }

            let mut inputs = base_inputs;
            inputs.volatility = vol;

            match price_caplet_floorlet(inputs) {
                Ok(price) => price.amount() - market_price,
                Err(_) => market_price, // High error on pricing failure
            }
        };

        // Solve for implied volatility using Brent solver
        let mut solver = BrentSolver::new().with_tolerance(1e-6);
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
