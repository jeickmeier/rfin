//! Calibration plan bumping utilities.
//!
//! These helpers mutate a `CalibrationPlan` in-place by bumping the quotes in the plan.
//! They are intended for risk/scenario workflows where you want to *re-run* calibration
//! under bumped quotes.

use super::BumpRequest;
use crate::calibration::api::schema::{CalibrationPlan, CalibrationStep, StepParams};
use crate::market::quotes::ids::Pillar;
use crate::market::quotes::market_quote::MarketQuote;
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
    pub fn bump(plan: &mut CalibrationPlan, bump: &BumpRequest) -> Result<()> {
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
    pub fn apply(mut plan: CalibrationPlan, bump: &BumpRequest) -> Result<CalibrationPlan> {
        Self::bump(&mut plan, bump)?;
        Ok(plan)
    }

    /// Bumps the quote set associated with a specific calibration step.
    fn bump_step_quote_set(
        plan: &mut CalibrationPlan,
        step: &CalibrationStep,
        bump: &BumpRequest,
    ) -> Result<()> {
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

/// Resolve the base date and day count for a step's time axis.
fn step_time_axis(step: &CalibrationStep) -> Result<(Date, DayCount)> {
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

/// Apply tenor-based bumps to a collection of market quotes.
fn bump_quotes_by_tenor(
    quotes: &mut [MarketQuote],
    base_date: Date,
    dc: DayCount,
    bump: &BumpRequest,
) {
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

/// Extract the maturity or expiry time (in years) for a market quote.
fn quote_time_years(
    q: &MarketQuote,
    base_date: Date,
    dc: DayCount,
    ctx: DayCountCtx,
) -> Option<f64> {
    let end = match q {
        MarketQuote::Rates(r) => match r {
            crate::market::quotes::rates::RateQuote::Deposit { pillar, .. } => {
                resolve_pillar(pillar, base_date)?
            }
            crate::market::quotes::rates::RateQuote::Fra { end, .. } => {
                resolve_pillar(end, base_date)?
            }
            crate::market::quotes::rates::RateQuote::Futures { expiry, .. } => *expiry,
            crate::market::quotes::rates::RateQuote::Swap { pillar, .. } => {
                resolve_pillar(pillar, base_date)?
            }
        },
        MarketQuote::Cds(c) => match c {
            crate::market::quotes::cds::CdsQuote::CdsParSpread { pillar, .. } => {
                resolve_pillar(pillar, base_date)?
            }
            crate::market::quotes::cds::CdsQuote::CdsUpfront { pillar, .. } => {
                resolve_pillar(pillar, base_date)?
            }
        },
        MarketQuote::CdsTranche(_) => return None, // Or handle if tranches have pillars
        MarketQuote::Inflation(i) => match i {
            crate::market::quotes::inflation::InflationQuote::InflationSwap {
                maturity, ..
            } => *maturity,
            crate::market::quotes::inflation::InflationQuote::YoYInflationSwap {
                maturity, ..
            } => *maturity,
        },
        MarketQuote::Vol(v) => match v {
            crate::market::quotes::vol::VolQuote::OptionVol { expiry, .. } => *expiry,
            crate::market::quotes::vol::VolQuote::SwaptionVol { expiry, .. } => *expiry,
        },
    };
    dc.year_fraction(base_date, end, ctx).ok()
}

fn resolve_pillar(pillar: &Pillar, base_date: Date) -> Option<Date> {
    match pillar {
        Pillar::Date(d) => Some(*d),
        Pillar::Tenor(t) => t
            .add_to_date(
                base_date,
                None,
                finstack_core::dates::BusinessDayConvention::Following,
            )
            .ok(),
    }
}

#[inline]
fn bp_to_decimal(bp: f64) -> f64 {
    bp * 1e-4
}
