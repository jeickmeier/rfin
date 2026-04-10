use super::config::CDSPricerConfig;
use super::helpers::{
    approx_default_density, df_asof_to, haz_t, midpoint_default_date,
    restructuring_adjustment_factor, settlement_date, sp_cond_to,
};
use super::IntegrationMethod;
use crate::constants::{isda, numerical, BASIS_POINTS_PER_UNIT};
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_core::dates::{Date, HolidayCalendar};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::adaptive_simpson;
use finstack_core::money::Money;
use finstack_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;

/// CDS pricing engine. Stateless wrapper carrying configuration.
#[derive(Debug)]
pub(crate) struct CDSPricer {
    pub(super) config: CDSPricerConfig,
}

#[derive(Clone, Copy)]
pub(super) struct AodInputs<'a> {
    pub(super) cds: &'a CreditDefaultSwap,
    pub(super) spread: f64,
    pub(super) start_date: Date,
    pub(super) end_date: Date,
    pub(super) settlement_delay: u16,
    pub(super) calendar: Option<&'a dyn HolidayCalendar>,
    pub(super) as_of: Date,
    pub(super) disc: &'a DiscountCurve,
    pub(super) surv: &'a HazardCurve,
}

#[derive(Clone, Copy)]
pub(super) struct CouponPeriod {
    pub(super) accrual_start: Date,
    pub(super) accrual_end: Date,
    pub(super) payment_date: Date,
}

impl Default for CDSPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSPricer {
    /// Create new pricer with default ISDA-compliant config.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
        }
    }

    /// Create pricer with custom config.
    ///
    /// Note: This method does not validate the configuration. For fail-fast
    /// validation, use [`try_with_config`](Self::try_with_config) instead.
    #[must_use]
    pub(crate) fn with_config(config: CDSPricerConfig) -> Self {
        Self { config }
    }

    /// Create pricer with custom config, validating parameters.
    ///
    /// Returns an error if the configuration contains invalid parameters.
    /// Prefer this over [`with_config`](Self::with_config) when configuration
    /// comes from external sources (user input, config files, etc.).
    ///
    /// # Errors
    ///
    /// Returns a validation error if the configuration is invalid.
    /// See [`CDSPricerConfig::validate`] for details.
    pub(crate) fn try_with_config(config: CDSPricerConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Get the configuration for this pricer.
    #[must_use]
    pub(crate) fn config(&self) -> &CDSPricerConfig {
        &self.config
    }

    /// Calculate PV of protection leg with numerical integration.
    ///
    /// The protection leg represents the contingent payment made by the
    /// protection seller upon a credit event. Its present value is:
    ///
    /// ```text
    /// PV_prot = (1 - R) × ∫ DF(t + delay) × (-dS(t)) dt
    /// ```
    ///
    /// where R is the recovery rate, DF is the discount factor, S is the
    /// survival probability, and delay is the settlement delay in years.
    /// Calculate PV of protection leg (Money)
    pub(crate) fn pv_protection_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_protection_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of protection leg (raw f64)
    ///
    /// Uses proper time-axis conventions:
    /// - Times are computed using the hazard curve's day-count convention
    /// - Survival probabilities are conditional on no default before `as_of`
    /// - Discounting uses the discount curve (times mapped from hazard curve axis)
    ///
    /// # Panics
    ///
    /// This method assumes the CDS has been validated at construction time.
    /// Recovery rate is expected to be in [0, 1]. Invalid recovery rates will
    /// produce incorrect results without error.
    pub(crate) fn pv_protection_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        // Note: Recovery rate validation is performed at CDS construction time.
        // All public constructors (builder, new_isda) call validate().

        // Protection leg covers the period from protection start to premium end.
        // For forward-starting CDS, protection begins at protection_effective_date
        // (which may be later than the premium leg start).
        // We only value protection from as_of onwards (can't protect against past defaults).
        let protection_start = as_of.max(cds.protection_start());
        let protection_end = cds.premium.end;

        // Forward-start at or past maturity: no protection interval by construction.
        if cds.protection_start() >= protection_end {
            return Ok(0.0);
        }

        // Determine the effective restructuring adjustment policy.
        // By default the pricer disables restructuring uplift because the current
        // implementation is only a heuristic approximation. It can be re-enabled
        // explicitly via `CDSPricerConfig::enable_restructuring_approximation`.
        let effective_clause = cds.doc_clause_effective();
        let restructuring_factor = if self.config.enable_restructuring_approximation {
            restructuring_adjustment_factor(effective_clause, cds)
        } else {
            1.0
        };

        // Use hazard curve's day-count for time axis (survival is the dominant factor)
        let t_asof = haz_t(surv, as_of)?;
        let t_start = haz_t(surv, protection_start)?;
        let t_end = haz_t(surv, protection_end)?;

        let recovery = cds.protection.recovery_rate;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        // Compute survival at as_of for conditioning
        let sp_asof = surv.sp(t_asof);

        let protection_pv = match self.config.integration_method {
            IntegrationMethod::Midpoint => self.protection_leg_midpoint_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
            IntegrationMethod::GaussianQuadrature => {
                match self.protection_leg_gaussian_quadrature_cond(
                    t_start,
                    t_end,
                    recovery,
                    cds.protection.settlement_delay,
                    calendar,
                    sp_asof,
                    as_of,
                    disc,
                    surv,
                ) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "GaussianQuadrature",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(
                            t_start,
                            t_end,
                            recovery,
                            cds.protection.settlement_delay,
                            calendar,
                            sp_asof,
                            as_of,
                            disc,
                            surv,
                        )?
                    }
                }
            }
            IntegrationMethod::AdaptiveSimpson => {
                match self.protection_leg_adaptive_simpson_cond(
                    t_start,
                    t_end,
                    recovery,
                    cds.protection.settlement_delay,
                    calendar,
                    sp_asof,
                    as_of,
                    disc,
                    surv,
                ) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "AdaptiveSimpson",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(
                            t_start,
                            t_end,
                            recovery,
                            cds.protection.settlement_delay,
                            calendar,
                            sp_asof,
                            as_of,
                            disc,
                            surv,
                        )?
                    }
                }
            }
            IntegrationMethod::IsdaExact => self.protection_leg_isda_exact_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
            IntegrationMethod::IsdaStandardModel => self.protection_leg_isda_standard_model_cond(
                t_start,
                t_end,
                recovery,
                cds.protection.settlement_delay,
                calendar,
                sp_asof,
                as_of,
                disc,
                surv,
            )?,
        };

        // Apply the restructuring adjustment. Contracts with restructuring as a
        // credit event (Cr14, Mr14, Mm14) have protection worth more than Xr14
        // because they cover an additional class of credit events.
        Ok(protection_pv * restructuring_factor * cds.notional.amount())
    }

    /// Calculate PV of premium leg with optional accrual-on-default
    /// Calculate PV of premium leg (Money)
    pub(crate) fn pv_premium_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let pv = self.pv_premium_leg_raw(cds, disc, surv, as_of)?;
        Ok(Money::new(pv, cds.notional.currency()))
    }

    /// Calculate PV of premium leg (raw f64)
    ///
    /// Uses proper time-axis conventions:
    /// - Discounting: relative DF from `as_of` using discount curve's day-count
    /// - Survival: conditional survival given no default before `as_of` using hazard curve's day-count
    /// - Accrual: instrument's premium leg day-count convention (Act/360 for NA, etc.)
    pub(crate) fn pv_premium_leg_raw(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);
        let periods = self.coupon_periods(cds, as_of)?;
        let spread = cds.premium.spread_bp.to_f64().ok_or_else(|| {
            Error::Validation("premium spread_bp cannot be represented as f64".into())
        })? / BASIS_POINTS_PER_UNIT;

        let mut premium_pv = 0.0;
        for period in periods {
            let start_date = period.accrual_start;
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            let accrual = cds.premium.day_count.year_fraction(
                start_date,
                end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let scheduled_coupon = cds.notional.amount() * spread * accrual;
            premium_pv += scheduled_coupon * sp * df;

            if self.config.include_accrual {
                let spread_sign = spread.signum();
                // Keep AoD on the same dollar basis as the scheduled coupon leg.
                premium_pv += spread_sign
                    * cds.notional.amount()
                    * self.accrual_on_default_isda_midpoint(AodInputs {
                        cds,
                        spread: spread.abs(),
                        start_date: start_date.max(as_of),
                        end_date,
                        settlement_delay: cds.protection.settlement_delay,
                        calendar,
                        as_of,
                        disc,
                        surv,
                    })?;
            }
        }

        Ok(premium_pv)
    }

    /// Calculate accrual-on-default for a period using dates with proper time-axis handling.
    ///
    /// This method properly handles:
    /// - Discounting using discount curve's day-count relative to as_of
    /// - Survival using hazard curve's day-count with conditional probability from as_of
    /// - Accrual fraction within the period
    pub(super) fn accrual_on_default_isda_midpoint(&self, inp: AodInputs<'_>) -> Result<f64> {
        // ISDA midpoint approximation for accrual-on-default, using **dates**:
        //
        // AoD ≈ spread * (0.5 * τ_remaining) * DF(as_of→pay) * P(default in (start, end] | survived to as_of)
        //
        // Important: `start_date` is already `max(period_start, as_of)` in all call sites,
        // so this implements a "clean" AoD (does not include already-accrued premium before `as_of`).
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }

        // Remaining accrual fraction uses the instrument premium day count convention.
        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;

        // Conditional default probability between start and end (conditioned on survival to as_of).
        let sp_start = sp_cond_to(inp.surv, inp.as_of, inp.start_date)?;
        let sp_end = sp_cond_to(inp.surv, inp.as_of, inp.end_date)?;
        let default_prob = (sp_start - sp_end).max(0.0);

        let default_date = midpoint_default_date(inp.surv, inp.start_date, inp.end_date)?;
        let settle_date = settlement_date(
            default_date,
            inp.settlement_delay,
            inp.calendar,
            self.config.business_days_per_year,
        )?;
        let df = df_asof_to(inp.disc, inp.as_of, settle_date)?;

        Ok(inp.spread * 0.5 * tau_remaining * default_prob * df)
    }

    /// Midpoint method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_midpoint_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        let num_steps = (period_length * self.config.steps_per_year as f64).ceil() as usize;
        let num_steps = num_steps.max(1);
        let dt_haz = period_length / num_steps as f64;
        // Assume disc and haz time axes are similar for step sizing (not exact but reasonable)
        let dt_disc = (t_start_disc - t_start_haz + period_length) / num_steps as f64;
        let _ = dt_disc; // We'll interpolate disc time from haz time ratio

        let mut accrual_pv = 0.0;
        for i in 0..num_steps {
            let t1_haz = t_start_haz + i as f64 * dt_haz;
            let t2_haz = t_start_haz + (i + 1) as f64 * dt_haz;

            // Conditional default probability for this sub-period
            let sp1 = surv.sp(t1_haz) / sp_asof;
            let sp2 = surv.sp(t2_haz) / sp_asof;
            let default_prob = sp1 - sp2;

            // Default assumed at midpoint
            let t_mid_haz = (t1_haz + t2_haz) * 0.5;
            // Map haz time to disc time (linear interpolation approximation)
            let ratio = (t_mid_haz - t_start_haz) / period_length;
            let t_mid_disc = t_start_disc + ratio * (t_start_disc - t_start_haz + period_length);

            // Relative DF from as_of
            let df = disc.df(t_mid_disc) / df_asof;

            // Accrued time within the period (from start to default)
            let accrued_fraction = t_mid_haz - t_start_haz;
            let accrual = spread * accrued_fraction;

            accrual_pv += accrual * default_prob * df;
        }
        Ok(accrual_pv)
    }

    /// Adaptive method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_adaptive_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        if period_length <= 0.0 || spread < 0.0 {
            return Ok(0.0);
        }

        let h = period_length * numerical::INTEGRATION_STEP_FACTOR;
        let integrand = |t_haz: f64| {
            // Density of default at t_haz (conditioned on survival to as_of)
            let density = approx_default_density(surv, t_haz, h, t_start_haz, t_end_haz) / sp_asof;

            // Map haz time to disc time
            let ratio = (t_haz - t_start_haz) / period_length;
            let t_disc = t_start_disc + ratio * period_length;

            // Relative DF from as_of
            let df = disc.df(t_disc) / df_asof;

            // Accrued time within period
            let accrued_time = (t_haz - t_start_haz).max(0.0);

            spread * accrued_time * density * df
        };

        adaptive_simpson(
            integrand,
            t_start_haz,
            t_end_haz,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    /// ISDA-style method for AoD with proper time-axis handling
    #[allow(clippy::too_many_arguments)]
    fn accrual_on_default_isda_dates(
        &self,
        spread: f64,
        t_start_haz: f64,
        t_end_haz: f64,
        t_start_disc: f64,
        _t_end_disc: f64,
        sp_asof: f64,
        df_asof: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end_haz - t_start_haz;
        if period_length <= 0.0 {
            return Ok(0.0);
        }

        // Use ISDA piecewise-constant approximation
        let steps_per_period = self.config.effective_steps(period_length);
        let dt = period_length / steps_per_period as f64;
        let mut accrual_pv = 0.0;

        for i in 0..steps_per_period {
            let t1_haz = t_start_haz + i as f64 * dt;
            let t2_haz = t1_haz + dt;

            // Conditional survival probabilities
            let sp1 = surv.sp(t1_haz) / sp_asof;
            let sp2 = surv.sp(t2_haz) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                // Note: Hazard rate computed for documentation but not needed in simplified formula
                // let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Map to disc time axis
                let ratio1 = (t1_haz - t_start_haz) / period_length;
                let t1_disc = t_start_disc + ratio1 * period_length;

                // Relative DF at interval start
                let df1 = disc.df(t1_disc) / df_asof;

                // Accrued time from period start to interval start
                let accrued_time = t1_haz - t_start_haz;

                // ISDA: accrual at default is approximately at interval midpoint
                // Simplified: use accrued_time + dt/2 as average
                let avg_accrued = accrued_time + dt * 0.5;

                // Contribution: spread * avg_accrued * (probability of default in interval) * df
                // Default prob = sp1 - sp2
                accrual_pv += spread * avg_accrued * (sp1 - sp2) * df1;
            }
        }

        Ok(accrual_pv)
    }

    /// Calculate accrual-on-default for a period using configured method (time-based)
    ///
    /// Note: This method assumes times are computed using consistent day-count conventions.
    /// Prefer using `calculate_accrual_on_default_dates` for new code.
    fn calculate_accrual_on_default(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let period_length = t_end - t_start;
        match self.config.integration_method {
            IntegrationMethod::Midpoint => {
                self.accrual_on_default_midpoint(spread, t_start, t_end, period_length, disc, surv)
            }
            IntegrationMethod::GaussianQuadrature | IntegrationMethod::AdaptiveSimpson => {
                self.accrual_on_default_adaptive(spread, t_start, t_end, period_length, disc, surv)
            }
            IntegrationMethod::IsdaExact => self.accrual_on_default_isda_exact(
                spread,
                t_start,
                t_end,
                period_length,
                disc,
                surv,
            ),
            IntegrationMethod::IsdaStandardModel => self.accrual_on_default_isda_standard_model(
                spread,
                t_start,
                t_end,
                period_length,
                disc,
                surv,
            ),
        }
    }

    fn accrual_on_default_midpoint(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let num_steps = (period_length * self.config.steps_per_year as f64).ceil() as usize;
        let dt = period_length / num_steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let default_prob = surv.sp(t1) - surv.sp(t2);
            let t_default = (t1 + t2) * 0.5;
            let accrued_time = t_default - t_start;
            let df = disc.df(t_default);
            let accrual = spread * accrued_time;
            accrual_pv += accrual * default_prob * df;
        }
        Ok(accrual_pv)
    }

    fn accrual_on_default_adaptive(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        _period_length: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end || spread < 0.0 {
            return Err(Error::internal(
                "accrued-on-default integral requires t_start < t_end and non-negative spread",
            ));
        }
        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let accrued_time = (t - t_start).max(0.0);
            let df = disc.df(t);
            spread * accrued_time * density * df
        };
        adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    fn accrual_on_default_isda_exact(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let steps = isda::STANDARD_INTEGRATION_POINTS;
        let dt = period_length / steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..steps {
            let t = t_start + (i as f64 + 0.5) * dt;
            let accrual_fraction = (t - t_start) / period_length;
            let t1 = t - dt * 0.5;
            let t2 = t + dt * 0.5;
            let sp1 = if t1 >= 0.0 { surv.sp(t1) } else { 1.0 };
            let sp2 = surv.sp(t2);
            let default_prob = if sp1 > 0.0 && sp2 < sp1 {
                (sp1 - sp2) / dt
            } else {
                0.0
            };
            let df = disc.df(t);
            accrual_pv += spread * accrual_fraction * default_prob * df * dt;
        }
        Ok(accrual_pv)
    }

    fn accrual_on_default_isda_standard_model(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        let steps = isda::STANDARD_INTEGRATION_POINTS;
        let dt = period_length / steps as f64;
        let mut accrual_pv = 0.0;

        for i in 0..steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = if t1 >= 0.0 { surv.sp(t1) } else { 1.0 };
            let sp2 = surv.sp(t2);

            if sp1 > 0.0 && sp2 < sp1 {
                // Calculate piecewise constant hazard rate
                let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Get discount factors at both ends
                let df1 = disc.df(t1);
                let df2 = disc.df(t2);

                // Calculate piecewise constant interest rate (allow negative rates)
                // Negative rates are valid when df2 > df1 (discount factors rising)
                let interest_rate = if df1 > 0.0 && df2 > 0.0 {
                    -(df2 / df1).ln() / dt
                } else {
                    0.0
                };

                // ISDA Standard Model analytical integration for accrual on default:
                // We need ∫[t1,t2] (t - t_start) * D(t) * λ * S(t) dt
                // where D(t) = D(t1) * exp(-r*(t-t1)) and S(t) = S(t1) * exp(-λ*(t-t1))
                //
                // Let τ = t - t1, then the integral becomes:
                // D(t1) * S(t1) * ∫[0,dt] (t_start - t1 + τ) * exp(-(r+λ)*τ) * λ dτ
                // = D(t1) * S(t1) * λ * [(t1 - t_start) * I0 + I1]
                // where I0 = ∫exp(-(r+λ)*τ)dτ and I1 = ∫τ*exp(-(r+λ)*τ)dτ

                let lambda_plus_r = hazard_rate + interest_rate;

                if lambda_plus_r.abs() > numerical::ZERO_TOLERANCE {
                    let exp_term = (-lambda_plus_r * dt).exp();
                    // I0 = [1 - exp(-k*dt)] / k
                    let i0 = (1.0 - exp_term) / lambda_plus_r;
                    // I1 = [1 - exp(-k*dt)*(1 + k*dt)] / k^2
                    let i1 = (1.0 - exp_term * (1.0 + lambda_plus_r * dt))
                        / (lambda_plus_r * lambda_plus_r);

                    accrual_pv += spread * df1 * sp1 * hazard_rate * ((t1 - t_start) * i0 + i1);
                } else {
                    // Fallback: midpoint approximation for very small rates
                    let t_mid = (t1 + t2) * 0.5;
                    let accrued_time = t_mid - t_start;
                    accrual_pv += spread * accrued_time * (sp1 - sp2) * df1;
                }
            }
        }

        Ok(accrual_pv)
    }
}
