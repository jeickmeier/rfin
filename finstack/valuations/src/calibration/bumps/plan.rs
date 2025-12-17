//! Calibration plan bumping utilities.
//!
//! These helpers mutate a `CalibrationPlanV2` in-place by bumping the quotes in the plan.
//! They are intended for risk/scenario workflows where you want to *re-run* calibration
//! under bumped quotes.

use super::BumpRequest;
use crate::calibration::api::schema::{CalibrationPlanV2, CalibrationStepV2, StepParams};
use crate::calibration::domain::quotes::MarketQuote;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::prelude::*;

/// Utilities for bumping calibration plans (risk analysis).
pub struct PlanBumper;

impl PlanBumper {
    /// Apply a bump request across the plan.
    ///
    /// - **Parallel**: bumps every quote in every quote set.
    /// - **Tenors**: bumps only quotes whose maturity/expiry time (in years from the step base date)
    ///   is within a small tolerance of the requested target tenor.
    ///
    /// Note: For tenor bumps we use each step's `base_date` and (where applicable) step conventions
    /// (e.g. rates curve day-count). If the same `quote_set` is referenced by multiple steps with
    /// different base dates, tenor bumps will apply once per referencing step.
    pub fn bump(plan: &mut CalibrationPlanV2, bump: &BumpRequest) -> Result<()> {
        match bump {
            BumpRequest::Parallel(bp) => {
                let amount = bp_to_decimal(*bp);
                for quotes in plan.quote_sets.values_mut() {
                    for q in quotes.iter_mut() {
                        *q = q.bump(amount);
                    }
                }
                Ok(())
            }
            BumpRequest::Tenors(_) => {
                // Tenor bumping needs a base date; we interpret tenors relative to each step.
                // Clone steps to avoid borrowing `plan` immutably (steps) and mutably (quote_sets)
                // at the same time.
                let steps = plan.steps.clone();
                for step in &steps {
                    Self::bump_step_quote_set(plan, step, bump)?;
                }
                Ok(())
            }
        }
    }

    /// Create a new plan with bump applied.
    pub fn apply(mut plan: CalibrationPlanV2, bump: &BumpRequest) -> Result<CalibrationPlanV2> {
        Self::bump(&mut plan, bump)?;
        Ok(plan)
    }

    fn bump_step_quote_set(plan: &mut CalibrationPlanV2, step: &CalibrationStepV2, bump: &BumpRequest) -> Result<()> {
        let quotes = plan.quote_sets.get_mut(&step.quote_set).ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: format!("Quote set '{}' not found", step.quote_set),
            })
        })?;

        let (base_date, dc) = step_time_axis(step)?;
        bump_quotes_by_tenor(quotes, base_date, dc, bump);
        Ok(())
    }
}

fn step_time_axis(step: &CalibrationStepV2) -> Result<(Date, DayCount)> {
    match &step.params {
        StepParams::Discount(p) => Ok((
            p.base_date,
            p.conventions.curve_day_count.unwrap_or(DayCount::Act365F),
        )),
        StepParams::Forward(p) => Ok((
            p.base_date,
            p.conventions.curve_day_count.unwrap_or(DayCount::Act365F),
        )),
        StepParams::Hazard(p) => Ok((p.base_date, DayCount::Act365F)),
        StepParams::Inflation(p) => Ok((p.base_date, DayCount::Act365F)),
        StepParams::VolSurface(p) => Ok((p.base_date, DayCount::Act365F)),
        StepParams::SwaptionVol(p) => Ok((p.base_date, DayCount::Act365F)),
        StepParams::BaseCorrelation(p) => Ok((p.base_date, DayCount::Act365F)),
    }
}

fn bump_quotes_by_tenor(quotes: &mut [MarketQuote], base_date: Date, dc: DayCount, bump: &BumpRequest) {
    let BumpRequest::Tenors(targets) = bump else {
        return;
    };

    // Tolerance for matching a bucket in years.
    let tol_years = 0.1_f64;
    let ctx = DayCountCtx::default();

    for q in quotes.iter_mut() {
        let Some(t) = quote_time_years(q, base_date, dc, ctx) else {
            continue;
        };

        let mut total_bp = 0.0_f64;
        for (target_t, bp) in targets {
            if (t - *target_t).abs() < tol_years {
                total_bp += *bp;
            }
        }
        if total_bp.abs() > 0.0 {
            *q = q.bump(bp_to_decimal(total_bp));
        }
    }
}

fn quote_time_years(
    q: &MarketQuote,
    base_date: Date,
    dc: DayCount,
    ctx: DayCountCtx,
) -> Option<f64> {
    let end = match q {
        MarketQuote::Rates(r) => r.maturity_date(),
        MarketQuote::Credit(c) => c.maturity_date()?,
        MarketQuote::Inflation(i) => i.maturity_date()?,
        MarketQuote::Vol(v) => match v {
            crate::calibration::domain::quotes::VolQuote::OptionVol { expiry, .. } => *expiry,
            crate::calibration::domain::quotes::VolQuote::SwaptionVol { expiry, .. } => *expiry,
        },
    };
    dc.year_fraction(base_date, end, ctx).ok()
}

#[inline]
fn bp_to_decimal(bp: f64) -> f64 {
    bp * 1e-4
}


