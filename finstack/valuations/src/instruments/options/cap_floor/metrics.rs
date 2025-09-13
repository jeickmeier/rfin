//! Interest rate option specific metrics calculators

use crate::instruments::options::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for interest rate options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Get market curves
        let disc_curve = context.curves.disc(option.disc_id)?;
        let fwd_curve = context.curves.fwd(option.forward_id)?;
        let base_date = disc_curve.base_date();

        // For caps/floors, aggregate delta across all caplets/floorlets
        if matches!(
            option.rate_option_type,
            super::RateOptionType::Cap | super::RateOptionType::Floor
        ) {
            use crate::cashflow::builder::schedule_utils::build_dates;
            use finstack_core::dates::{BusinessDayConvention, StubKind};

            let schedule = build_dates(
                option.start_date,
                option.end_date,
                option.frequency,
                StubKind::None,
                BusinessDayConvention::Following,
                None,
            );

            let mut total_delta = 0.0;
            let mut prev_date = schedule.dates[0];

            for &payment_date in &schedule.dates[1..] {
                let time_to_fixing = option.day_count.year_fraction(
                    base_date,
                    prev_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let time_to_payment = option.day_count.year_fraction(
                    base_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let period_length = option.day_count.year_fraction(
                    prev_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                if time_to_fixing > 0.0 {
                    let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
                    let df = disc_curve.df(time_to_payment);

                    let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                        impl_vol
                    } else {
                        context
                            .curves
                            .surface(option.vol_id)?
                            .value_clamped(time_to_fixing, option.strike_rate)
                    };

                    let caplet_delta = option.delta(forward_rate, sigma, time_to_fixing);
                    total_delta += caplet_delta * option.notional.amount() * period_length * df;
                }
                prev_date = payment_date;
            }

            Ok(total_delta)
        } else {
            // Single caplet/floorlet
            let time_to_fixing = option.day_count.year_fraction(
                base_date,
                option.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let time_to_payment = option.day_count.year_fraction(
                base_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let period_length = option.day_count.year_fraction(
                option.start_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            if time_to_fixing <= 0.0 {
                return Ok(0.0);
            }

            let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
            let df = disc_curve.df(time_to_payment);

            let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                impl_vol
            } else {
                context
                    .curves
                    .surface(option.vol_id)?
                    .value_clamped(time_to_fixing, option.strike_rate)
            };

            let delta = option.delta(forward_rate, sigma, time_to_fixing);
            Ok(delta * option.notional.amount() * period_length * df)
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for interest rate options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Similar aggregation logic as Delta but for Gamma
        let disc_curve = context.curves.disc(option.disc_id)?;
        let fwd_curve = context.curves.fwd(option.forward_id)?;
        let base_date = disc_curve.base_date();

        if matches!(
            option.rate_option_type,
            super::RateOptionType::Cap | super::RateOptionType::Floor
        ) {
            use crate::cashflow::builder::schedule_utils::build_dates;
            use finstack_core::dates::{BusinessDayConvention, StubKind};

            let schedule = build_dates(
                option.start_date,
                option.end_date,
                option.frequency,
                StubKind::None,
                BusinessDayConvention::Following,
                None,
            );

            let mut total_gamma = 0.0;
            let mut prev_date = schedule.dates[0];

            for &payment_date in &schedule.dates[1..] {
                let time_to_fixing = option.day_count.year_fraction(
                    base_date,
                    prev_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let time_to_payment = option.day_count.year_fraction(
                    base_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let period_length = option.day_count.year_fraction(
                    prev_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                if time_to_fixing > 0.0 {
                    let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
                    let df = disc_curve.df(time_to_payment);

                    let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                        impl_vol
                    } else {
                        context
                            .curves
                            .surface(option.vol_id)?
                            .value_clamped(time_to_fixing, option.strike_rate)
                    };

                    let caplet_gamma = option.gamma(forward_rate, sigma, time_to_fixing);
                    total_gamma += caplet_gamma * option.notional.amount() * period_length * df;
                }
                prev_date = payment_date;
            }

            Ok(total_gamma)
        } else {
            let time_to_fixing = option.day_count.year_fraction(
                base_date,
                option.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let time_to_payment = option.day_count.year_fraction(
                base_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let period_length = option.day_count.year_fraction(
                option.start_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            if time_to_fixing <= 0.0 {
                return Ok(0.0);
            }

            let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
            let df = disc_curve.df(time_to_payment);

            let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                impl_vol
            } else {
                context
                    .curves
                    .surface(option.vol_id)?
                    .value_clamped(time_to_fixing, option.strike_rate)
            };

            let gamma = option.gamma(forward_rate, sigma, time_to_fixing);
            Ok(gamma * option.notional.amount() * period_length * df)
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for interest rate options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &InterestRateOption = context.instrument_as()?;
        let disc_curve = context.curves.disc(option.disc_id)?;
        let fwd_curve = context.curves.fwd(option.forward_id)?;
        let base_date = disc_curve.base_date();

        if matches!(
            option.rate_option_type,
            super::RateOptionType::Cap | super::RateOptionType::Floor
        ) {
            use crate::cashflow::builder::schedule_utils::build_dates;
            use finstack_core::dates::{BusinessDayConvention, StubKind};

            let schedule = build_dates(
                option.start_date,
                option.end_date,
                option.frequency,
                StubKind::None,
                BusinessDayConvention::Following,
                None,
            );

            let mut total_vega = 0.0;
            let mut prev_date = schedule.dates[0];

            for &payment_date in &schedule.dates[1..] {
                let time_to_fixing = option.day_count.year_fraction(
                    base_date,
                    prev_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let time_to_payment = option.day_count.year_fraction(
                    base_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let period_length = option.day_count.year_fraction(
                    prev_date,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                if time_to_fixing > 0.0 {
                    let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
                    let df = disc_curve.df(time_to_payment);

                    let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                        impl_vol
                    } else {
                        context
                            .curves
                            .surface(option.vol_id)?
                            .value_clamped(time_to_fixing, option.strike_rate)
                    };

                    let caplet_vega = option.vega(forward_rate, sigma, time_to_fixing);
                    total_vega += caplet_vega * option.notional.amount() * period_length * df;
                }
                prev_date = payment_date;
            }
            Ok(total_vega)
        } else {
            let time_to_fixing = option.day_count.year_fraction(
                base_date,
                option.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let time_to_payment = option.day_count.year_fraction(
                base_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let period_length = option.day_count.year_fraction(
                option.start_date,
                option.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;

            if time_to_fixing <= 0.0 {
                return Ok(0.0);
            }

            let forward_rate = fwd_curve.rate_period(time_to_fixing, time_to_payment);
            let df = disc_curve.df(time_to_payment);

            let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
                impl_vol
            } else {
                context
                    .curves
                    .surface(option.vol_id)?
                    .value_clamped(time_to_fixing, option.strike_rate)
            };

            let vega = option.vega(forward_rate, sigma, time_to_fixing);
            Ok(vega * option.notional.amount() * period_length * df)
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for interest rate options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;

        // For IR options, theta is typically calculated via finite difference
        // using a 1-day time bump on the pricing function
        let base_pv = context.base_value.amount();

        // Approximate theta as -dPV/dt per day
        // This is a simplified approach; full implementation would reprice with t-1day
        let dt = 1.0 / 365.25;
        let approx_theta = -base_pv * 0.01 * dt; // Rough approximation

        Ok(approx_theta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for interest rate options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;
        // Placeholder: rho requires rate bump; not available here
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Implied Volatility calculator for interest rate options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register interest rate option metrics with the registry
pub fn register_interest_rate_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Vega,
        Arc::new(VegaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::Rho,
        Arc::new(RhoCalculator),
        &["InterestRateOption"],
    );

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["InterestRateOption"],
    );
}
