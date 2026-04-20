use super::engine::CDSPricer;
use super::helpers::{
    approx_default_density, date_from_hazard_time, df_asof_to, isda_standard_model_boundaries,
    settlement_date,
};
use crate::constants::{credit, numerical};
use finstack_core::dates::{Date, HolidayCalendar};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::{Error, Result};
use std::cell::RefCell;

impl CDSPricer {
    /// Midpoint method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    pub(super) fn protection_leg_midpoint_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let tenor_years = t_end - t_start;
        let steps_per_year = self.config.effective_steps(tenor_years);
        let num_steps = ((tenor_years) * steps_per_year as f64).ceil() as usize;
        let num_steps = num_steps.max(1);
        let dt = tenor_years / num_steps as f64;
        let lgd = 1.0 - recovery;
        let mut protection_pv = 0.0;

        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let t_mid = (t1 + t2) * 0.5;

            // Conditional survival probabilities. Clamp the default probability
            // at zero: a strictly monotone hazard curve guarantees sp1 >= sp2,
            // but numerical roundoff or a mildly non-monotone user curve can
            // make the difference slightly negative, which would otherwise
            // contribute a spurious *positive* cash-flow to the protection leg.
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;
            let default_prob = (sp1 - sp2).max(0.0);

            // Discount on actual dates (supports discount/hazard curves with different day-counts).
            let default_date = date_from_hazard_time(surv, t_mid);
            let settle_date = settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            )?;
            let df = df_asof_to(disc, as_of, settle_date)?;

            protection_pv += lgd * default_prob * df;
        }
        Ok(protection_pv)
    }

    /// Gaussian quadrature method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    pub(super) fn protection_leg_gaussian_quadrature_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let df_error: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if df_error.borrow().is_some() {
                return 0.0;
            }
            // Conditional default density
            let density = approx_default_density(surv, t, h, t_start, t_end) / sp_asof;
            // Discount on actual dates
            let default_date = date_from_hazard_time(surv, t);
            let settle_date = match settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            ) {
                Ok(date) => date,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(disc, as_of, settle_date) {
                Ok(df) => df,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    0.0
                }
            };
            lgd * density * df
        };
        let result =
            gauss_legendre_integrate(integrand, t_start, t_end, self.config.validated_gl_order())?;
        if let Some(err) = df_error.into_inner() {
            return Err(err);
        }
        Ok(result)
    }

    /// Adaptive Simpson method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    pub(super) fn protection_leg_adaptive_simpson_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let h = (t_end - t_start) * numerical::INTEGRATION_STEP_FACTOR;
        let lgd = 1.0 - recovery;
        let df_error: RefCell<Option<Error>> = RefCell::new(None);
        let integrand = |t: f64| {
            if df_error.borrow().is_some() {
                return 0.0;
            }
            // Conditional default density
            let density = approx_default_density(surv, t, h, t_start, t_end) / sp_asof;
            // Discount on actual dates
            let default_date = date_from_hazard_time(surv, t);
            let settle_date = match settlement_date(
                default_date,
                settlement_delay,
                calendar,
                self.config.business_days_per_year,
            ) {
                Ok(date) => date,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    return 0.0;
                }
            };
            let df = match df_asof_to(disc, as_of, settle_date) {
                Ok(df) => df,
                Err(e) => {
                    *df_error.borrow_mut() = Some(e);
                    0.0
                }
            };
            lgd * density * df
        };
        let result = adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )?;
        if let Some(err) = df_error.into_inner() {
            return Err(err);
        }
        Ok(result)
    }

    /// ISDA exact method with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    pub(super) fn protection_leg_isda_exact_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let lgd = 1.0 - recovery;
        let tenor_years = t_end - t_start;
        let steps_per_period = self.config.effective_steps(tenor_years);
        let dt = tenor_years / steps_per_period as f64;
        let mut integral = 0.0;

        for i in 0..steps_per_period {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;

            // Conditional survival probabilities
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                let hazard_rate = -(sp2 / sp1).ln() / dt;
                let avg_t = (t1 + t2) * 0.5;
                let default_date = date_from_hazard_time(surv, avg_t);
                let settle_date = settlement_date(
                    default_date,
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let df_mid = df_asof_to(disc, as_of, settle_date)?;

                if hazard_rate.abs() > numerical::ZERO_TOLERANCE {
                    integral += (sp1 - sp2) * df_mid;
                } else {
                    let sp_mid = (sp1 + sp2) * 0.5;
                    integral += sp_mid * df_mid * hazard_rate * dt;
                }
            }
        }
        Ok(integral * lgd)
    }

    /// ISDA Standard Model with conditional survival and relative discounting
    #[allow(clippy::too_many_arguments)]
    pub(super) fn protection_leg_isda_standard_model_cond(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        settlement_delay: u16,
        calendar: Option<&dyn HolidayCalendar>,
        sp_asof: f64,
        as_of: Date,
        disc: &DiscountCurve,
        surv: &HazardCurve,
    ) -> Result<f64> {
        if t_start >= t_end {
            return Err(Error::Validation(format!(
                "Protection leg start time ({}) must be before end time ({})",
                t_start, t_end
            )));
        }
        if sp_asof <= credit::SURVIVAL_PROBABILITY_FLOOR {
            return Ok(0.0);
        }

        let lgd = 1.0 - recovery;
        let boundaries = isda_standard_model_boundaries(t_start, t_end, surv, disc);
        let mut integral = 0.0;

        for window in boundaries.windows(2) {
            let t1 = window[0];
            let t2 = window[1];
            let dt = t2 - t1;
            if dt <= numerical::ZERO_TOLERANCE {
                continue;
            }

            // Conditional survival probabilities
            let sp1 = surv.sp(t1) / sp_asof;
            let sp2 = surv.sp(t2) / sp_asof;

            if sp1 > sp2 && sp1 > 0.0 {
                // Piecewise constant hazard rate for this interval
                let hazard_rate = -(sp2 / sp1).ln() / dt;

                // Relative discount factors from as_of
                let d1 = settlement_date(
                    date_from_hazard_time(surv, t1),
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let d2 = settlement_date(
                    date_from_hazard_time(surv, t2),
                    settlement_delay,
                    calendar,
                    self.config.business_days_per_year,
                )?;
                let df1 = df_asof_to(disc, as_of, d1)?;
                let df2 = df_asof_to(disc, as_of, d2)?;

                // Piecewise constant interest rate (allow negative rates)
                let interest_rate = if df1 > 0.0 && df2 > 0.0 {
                    -(df2 / df1).ln() / dt
                } else {
                    0.0
                };

                // ISDA Standard Model analytical integration
                let lambda_plus_r = hazard_rate + interest_rate;

                if lambda_plus_r.abs() > numerical::ZERO_TOLERANCE {
                    let exp_term = (-lambda_plus_r * dt).exp();
                    integral += df1 * sp1 * (hazard_rate / lambda_plus_r) * (1.0 - exp_term);
                } else {
                    integral += df1 * sp1 * hazard_rate * dt;
                }
            }
        }

        Ok(integral * lgd)
    }
}
