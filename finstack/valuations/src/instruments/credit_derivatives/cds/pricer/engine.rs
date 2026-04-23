use super::config::CDSPricerConfig;
use super::helpers::{
    approx_default_density, date_from_hazard_time, df_asof_to, haz_t,
    isda_standard_model_boundaries, midpoint_default_date, restructuring_adjustment_factor,
    settlement_date, sp_cond_to,
};
use super::IntegrationMethod;
use crate::constants::{credit, numerical, BASIS_POINTS_PER_UNIT};
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_core::dates::{Date, HolidayCalendar};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::money::Money;
use finstack_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;
use std::cell::RefCell;

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

        let inputs = super::integration::ProtectionLegInputs {
            t_start,
            t_end,
            recovery,
            settlement_delay: cds.protection.settlement_delay,
            calendar,
            sp_asof,
            as_of,
            disc,
            surv,
        };
        let protection_pv = match self.config.integration_method {
            IntegrationMethod::Midpoint => self.protection_leg_midpoint_cond(&inputs)?,
            IntegrationMethod::GaussianQuadrature => {
                match self.protection_leg_gaussian_quadrature_cond(&inputs) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "GaussianQuadrature",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(&inputs)?
                    }
                }
            }
            IntegrationMethod::AdaptiveSimpson => {
                match self.protection_leg_adaptive_simpson_cond(&inputs) {
                    Ok(pv) => pv,
                    Err(e) => {
                        tracing::warn!(
                            method = "AdaptiveSimpson",
                            error = %e,
                            t_start = t_start,
                            t_end = t_end,
                            "Integration failed, falling back to midpoint method"
                        );
                        self.protection_leg_midpoint_cond(&inputs)?
                    }
                }
            }
            IntegrationMethod::IsdaExact => self.protection_leg_isda_exact_cond(&inputs)?,
            IntegrationMethod::IsdaStandardModel => {
                self.protection_leg_isda_standard_model_cond(&inputs)?
            }
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
    /// - Accrual-on-default: dispatched through [`Self::accrual_on_default_dispatch`]
    ///   which honours [`CDSPricerConfig::integration_method`].
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
                finstack_core::dates::DayCountContext::default(),
            )?;
            let scheduled_coupon = cds.notional.amount() * spread * accrual;
            premium_pv += scheduled_coupon * sp * df;

            if self.config.include_accrual {
                let spread_sign = spread.signum();
                // Keep AoD on the same dollar basis as the scheduled coupon leg.
                premium_pv += spread_sign
                    * cds.notional.amount()
                    * self.accrual_on_default_dispatch(AodInputs {
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

    // ─── Accrual-on-default dispatch and variants ─────────────────────────

    /// Route accrual-on-default to a method-specific implementation based on
    /// [`CDSPricerConfig::integration_method`].
    ///
    /// All implementations use **conditional survival** given no default before
    /// `as_of` and **relative discount factors** from `as_of`, matching the
    /// protection-leg conventions.
    ///
    /// | Method | AoD implementation |
    /// |--------|--------------------|
    /// | `Midpoint` | Period midpoint with conditional survival |
    /// | `IsdaExact` | Piecewise fixed-step midpoint quadrature |
    /// | `IsdaStandardModel` | Analytical piecewise-constant integration over hazard/disc knots |
    /// | `GaussianQuadrature` | Gauss-Legendre on hazard-time axis with conditional density |
    /// | `AdaptiveSimpson` | Adaptive Simpson on hazard-time axis with conditional density |
    pub(super) fn accrual_on_default_dispatch(&self, inp: AodInputs<'_>) -> Result<f64> {
        match self.config.integration_method {
            IntegrationMethod::Midpoint => self.accrual_on_default_midpoint_cond(inp),
            IntegrationMethod::IsdaExact => self.accrual_on_default_isda_exact_cond(inp),
            IntegrationMethod::IsdaStandardModel => {
                self.accrual_on_default_isda_standard_model_cond(inp)
            }
            IntegrationMethod::GaussianQuadrature => {
                match self.accrual_on_default_gaussian_quadrature_cond(inp) {
                    Ok(pv) => Ok(pv),
                    Err(e) => {
                        tracing::warn!(
                            method = "GaussianQuadrature",
                            error = %e,
                            "AoD integration failed, falling back to midpoint"
                        );
                        self.accrual_on_default_midpoint_cond(inp)
                    }
                }
            }
            IntegrationMethod::AdaptiveSimpson => {
                match self.accrual_on_default_adaptive_simpson_cond(inp) {
                    Ok(pv) => Ok(pv),
                    Err(e) => {
                        tracing::warn!(
                            method = "AdaptiveSimpson",
                            error = %e,
                            "AoD integration failed, falling back to midpoint"
                        );
                        self.accrual_on_default_midpoint_cond(inp)
                    }
                }
            }
        }
    }

    /// Period midpoint approximation for AoD with conditional survival.
    ///
    /// ```text
    /// AoD ≈ spread * (0.5 * τ_remaining) * DF(as_of→settle(mid))
    ///     * P(default in (start, end] | survived to as_of)
    /// ```
    ///
    /// `start_date` is already `max(period_start, as_of)` at the call site,
    /// so this implements a "clean" AoD (no already-accrued premium before
    /// `as_of`).
    pub(super) fn accrual_on_default_midpoint_cond(&self, inp: AodInputs<'_>) -> Result<f64> {
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }

        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;

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

    /// Piecewise fixed-step midpoint quadrature over [start, end] (ISDA exact
    /// variant), using conditional survival and relative discount factors.
    pub(super) fn accrual_on_default_isda_exact_cond(&self, inp: AodInputs<'_>) -> Result<f64> {
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }
        let t_start = haz_t(inp.surv, inp.start_date)?;
        let t_end = haz_t(inp.surv, inp.end_date)?;
        let t_asof = haz_t(inp.surv, inp.as_of)?;
        let sp_asof = inp.surv.sp(t_asof);
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;
        let period_length_haz = t_end - t_start;
        if period_length_haz <= 0.0 || tau_remaining <= 0.0 {
            return Ok(0.0);
        }

        let steps = self.config.effective_steps(period_length_haz).max(1);
        let dt = period_length_haz / steps as f64;
        let mut accrual_pv = 0.0;

        for i in 0..steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = inp.surv.sp(t1) / sp_asof;
            let sp2 = inp.surv.sp(t2) / sp_asof;
            if !(sp1 > sp2 && sp1 > 0.0) {
                continue;
            }
            // Default assumed at interval midpoint; accrual fraction scaled by
            // position within the accrual period on the instrument day-count.
            let t_mid = (t1 + t2) * 0.5;
            let position = ((t_mid - t_start) / period_length_haz).clamp(0.0, 1.0);
            let accrued_tau = tau_remaining * position;

            let default_date = date_from_hazard_time(inp.surv, t_mid);
            let settle_date = settlement_date(
                default_date,
                inp.settlement_delay,
                inp.calendar,
                self.config.business_days_per_year,
            )?;
            let df = df_asof_to(inp.disc, inp.as_of, settle_date)?;

            accrual_pv += inp.spread * accrued_tau * (sp1 - sp2) * df;
        }
        Ok(accrual_pv)
    }

    /// ISDA Standard Model AoD: analytical integration over piecewise-constant
    /// hazard and interest rate intervals (knot-aligned), using conditional
    /// survival and relative discount factors.
    pub(super) fn accrual_on_default_isda_standard_model_cond(
        &self,
        inp: AodInputs<'_>,
    ) -> Result<f64> {
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }
        let t_start = haz_t(inp.surv, inp.start_date)?;
        let t_end = haz_t(inp.surv, inp.end_date)?;
        let t_asof = haz_t(inp.surv, inp.as_of)?;
        let sp_asof = inp.surv.sp(t_asof);
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;
        let period_length_haz = t_end - t_start;
        if period_length_haz <= 0.0 || tau_remaining <= 0.0 {
            return Ok(0.0);
        }
        // Linear scale from hazard-time position to instrument-day-count accrual.
        let tau_per_haz = tau_remaining / period_length_haz;

        let boundaries = isda_standard_model_boundaries(t_start, t_end, inp.surv, inp.disc);
        let mut accrual_pv = 0.0;

        for window in boundaries.windows(2) {
            let t1 = window[0];
            let t2 = window[1];
            let dt = t2 - t1;
            if dt <= numerical::ZERO_TOLERANCE {
                continue;
            }

            let sp1 = inp.surv.sp(t1) / sp_asof;
            let sp2 = inp.surv.sp(t2) / sp_asof;
            if !(sp1 > sp2 && sp1 > 0.0) {
                continue;
            }

            // Piecewise-constant hazard rate over [t1, t2].
            let hazard_rate = -(sp2 / sp1).ln() / dt;

            // Relative DF anchored at as_of, via settled default date per knot.
            let settle1 = settlement_date(
                date_from_hazard_time(inp.surv, t1),
                inp.settlement_delay,
                inp.calendar,
                self.config.business_days_per_year,
            )?;
            let settle2 = settlement_date(
                date_from_hazard_time(inp.surv, t2),
                inp.settlement_delay,
                inp.calendar,
                self.config.business_days_per_year,
            )?;
            let df1 = df_asof_to(inp.disc, inp.as_of, settle1)?;
            let df2 = df_asof_to(inp.disc, inp.as_of, settle2)?;

            // Piecewise-constant interest rate (may be negative if df2 > df1).
            let interest_rate = if df1 > 0.0 && df2 > 0.0 {
                -(df2 / df1).ln() / dt
            } else {
                0.0
            };

            // Accrued fraction at interval start, expressed in instrument-DC units.
            let tau_at_t1 = (t1 - t_start) * tau_per_haz;

            // Analytical integration for
            //   ∫ spread * (τ_at_t1 + (t - t1) * tau_per_haz) * λ * S(t1) * D(t1)
            //     * exp(-(λ + r)(t - t1)) dt
            // Let k = λ + r. Then
            //   I0 = (1 - e^{-kΔ})/k
            //   I1 = (1 - e^{-kΔ}(1 + kΔ))/k²
            let k = hazard_rate + interest_rate;
            let contribution = if k.abs() > numerical::ZERO_TOLERANCE {
                let exp_term = (-k * dt).exp();
                let i0 = (1.0 - exp_term) / k;
                let i1 = (1.0 - exp_term * (1.0 + k * dt)) / (k * k);
                inp.spread * df1 * sp1 * hazard_rate * (tau_at_t1 * i0 + tau_per_haz * i1)
            } else {
                // Small-k fallback: midpoint approximation keeps AoD well-behaved
                // for near-zero hazard or near-zero (r+λ).
                let t_mid = (t1 + t2) * 0.5;
                let position = ((t_mid - t_start) / period_length_haz).clamp(0.0, 1.0);
                let accrued_tau = tau_remaining * position;
                inp.spread * accrued_tau * (sp1 - sp2) * df1
            };
            accrual_pv += contribution;
        }
        Ok(accrual_pv)
    }

    /// Gauss-Legendre quadrature on hazard-time axis with conditional density
    /// and relative discount factors.
    pub(super) fn accrual_on_default_gaussian_quadrature_cond(
        &self,
        inp: AodInputs<'_>,
    ) -> Result<f64> {
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }
        let t_start = haz_t(inp.surv, inp.start_date)?;
        let t_end = haz_t(inp.surv, inp.end_date)?;
        if t_start >= t_end {
            return Ok(0.0);
        }
        let t_asof = haz_t(inp.surv, inp.as_of)?;
        let sp_asof = inp.surv.sp(t_asof);
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;
        let period_length_haz = t_end - t_start;
        if tau_remaining <= 0.0 || period_length_haz <= 0.0 {
            return Ok(0.0);
        }
        let tau_per_haz = tau_remaining / period_length_haz;

        let h = period_length_haz * numerical::INTEGRATION_STEP_FACTOR;
        let err: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if err.borrow().is_some() {
                return 0.0;
            }
            let density = approx_default_density(inp.surv, t, h, t_start, t_end) / sp_asof;
            let default_date = date_from_hazard_time(inp.surv, t);
            let settle = match settlement_date(
                default_date,
                inp.settlement_delay,
                inp.calendar,
                self.config.business_days_per_year,
            ) {
                Ok(d) => d,
                Err(e) => {
                    *err.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(inp.disc, inp.as_of, settle) {
                Ok(d) => d,
                Err(e) => {
                    *err.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let tau = (t - t_start).max(0.0) * tau_per_haz;
            inp.spread * tau * density * df
        };

        let result =
            gauss_legendre_integrate(integrand, t_start, t_end, self.config.validated_gl_order())?;
        if let Some(e) = err.into_inner() {
            return Err(e);
        }
        Ok(result)
    }

    /// Adaptive Simpson on hazard-time axis with conditional density and
    /// relative discount factors.
    pub(super) fn accrual_on_default_adaptive_simpson_cond(
        &self,
        inp: AodInputs<'_>,
    ) -> Result<f64> {
        if inp.end_date <= inp.start_date {
            return Ok(0.0);
        }
        let t_start = haz_t(inp.surv, inp.start_date)?;
        let t_end = haz_t(inp.surv, inp.end_date)?;
        if t_start >= t_end {
            return Ok(0.0);
        }
        let t_asof = haz_t(inp.surv, inp.as_of)?;
        let sp_asof = inp.surv.sp(t_asof);
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tau_remaining = year_fraction(inp.cds.premium.day_count, inp.start_date, inp.end_date)?;
        let period_length_haz = t_end - t_start;
        if tau_remaining <= 0.0 || period_length_haz <= 0.0 {
            return Ok(0.0);
        }
        let tau_per_haz = tau_remaining / period_length_haz;

        let h = period_length_haz * numerical::INTEGRATION_STEP_FACTOR;
        let err: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if err.borrow().is_some() {
                return 0.0;
            }
            let density = approx_default_density(inp.surv, t, h, t_start, t_end) / sp_asof;
            let default_date = date_from_hazard_time(inp.surv, t);
            let settle = match settlement_date(
                default_date,
                inp.settlement_delay,
                inp.calendar,
                self.config.business_days_per_year,
            ) {
                Ok(d) => d,
                Err(e) => {
                    *err.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(inp.disc, inp.as_of, settle) {
                Ok(d) => d,
                Err(e) => {
                    *err.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let tau = (t - t_start).max(0.0) * tau_per_haz;
            inp.spread * tau * density * df
        };

        let result = adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )?;
        if let Some(e) = err.into_inner() {
            return Err(e);
        }
        Ok(result)
    }
}
