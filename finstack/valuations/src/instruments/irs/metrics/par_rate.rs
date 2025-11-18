//! IRS par rate metric.
//!
//! Computes the fixed rate that sets the swap PV to zero given curves.
//! Uses float-leg PV divided by notional times fixed-leg annuity.

use crate::instruments::{irs::ParRateMethod, InterestRateSwap};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;

/// Minimum threshold for discount factor values to avoid numerical instability.
/// Same as in pricer.rs to ensure consistency across IRS calculations.
const DF_EPSILON: f64 = 1e-10;

/// Basis points to decimal conversion factor.
const BP_TO_DECIMAL: f64 = 1e-4;

/// Par rate calculator for IRS.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;

        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;
        let base = disc.base_date();

        let method = irs.fixed.par_method.unwrap_or(ParRateMethod::ForwardBased);
        match method {
            ParRateMethod::ForwardBased => {
                // float PV / (N * annuity)
                let fwd = context.curves.get_forward(&irs.float.forward_curve_id)?;
                let as_of = context.as_of;

                // Annuity is sum(yf*df) in years
                let annuity = context
                    .computed
                    .get(&MetricId::Annuity)
                    .copied()
                    .unwrap_or(0.0); // This is fine - it's from a hashmap, not a calculation
                if annuity == 0.0 {
                    return Ok(0.0);
                }

                let fs = crate::cashflow::builder::build_dates(
                    irs.float.start,
                    irs.float.end,
                    irs.float.freq,
                    irs.float.stub,
                    irs.float.bdc,
                    irs.float.calendar_id.as_deref(),
                );
                let schedule: Vec<Date> = fs.dates;
                if schedule.len() < 2 {
                    return Ok(0.0);
                }

                let disc_dc = disc.day_count();
                let t_as_of = disc_dc
                    .year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())?;
                let df_as_of = disc.df(t_as_of);

                // Guard against near-zero discount factors for numerical stability
                if df_as_of.abs() < DF_EPSILON {
                    return Err(finstack_core::error::Error::Validation(format!(
                        "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
                         This may indicate extreme rate scenarios or very long time horizons.",
                        df_as_of, DF_EPSILON
                    )));
                }

                let mut pv = 0.0;
                let mut prev = schedule[0];
                for &d in &schedule[1..] {
                    // Only include future cashflows
                    if d <= as_of {
                        prev = d;
                        continue;
                    }

                    let t1 = irs
                        .float
                        .dc
                        .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())?;
                    let t2 = irs
                        .float
                        .dc
                        .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())?;
                    let yf = irs
                        .float
                        .dc
                        .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;

                    // Only call rate_period if t1 < t2 to avoid date ordering errors
                    let f = if t2 > t1 {
                        fwd.rate_period(t1, t2)
                    } else {
                        0.0
                    };
                    let rate = f + (irs.float.spread_bp * BP_TO_DECIMAL);
                    let coupon = irs.notional.amount() * rate * yf;

                    // Discount from as_of for correct theta and seasoned swap handling
                    let t_d = disc_dc
                        .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())?;
                    let df_d_abs = disc.df(t_d);
                    // df_as_of already validated above, safe to divide
                    let df = df_d_abs / df_as_of;

                    pv += coupon * df;
                    prev = d;
                }

                // Par rate = float_pv / (notional * annuity)
                // Annuity is sum(yf*df), so this gives: pv / (notional * sum(yf*df))
                Ok(pv / (irs.notional.amount() * annuity))
            }
            ParRateMethod::DiscountRatio => {
                // (P(as_of,T0) - P(as_of,Tn)) / Sum alpha_i P(as_of,Ti)
                // This formulation is only exact for unseasoned swaps where
                // `as_of` is on or before the fixed leg start date.
                let as_of = context.as_of;
                let sched = crate::cashflow::builder::build_dates(
                    irs.fixed.start,
                    irs.fixed.end,
                    irs.fixed.freq,
                    irs.fixed.stub,
                    irs.fixed.bdc,
                    irs.fixed.calendar_id.as_deref(),
                );
                let dates: Vec<Date> = sched.dates;
                if dates.len() < 2 {
                    return Ok(0.0);
                }

                // Guard against seasoned swaps: for `as_of` after the start date
                // the classic discount-ratio formula ceases to be exact. For live
                // trades use `ParRateMethod::ForwardBased` instead.
                if as_of > dates[0] {
                    return Err(finstack_core::error::Error::Validation(
                        format!(
                            "ParRateMethod::DiscountRatio requires as_of ({}) <= start_date ({}). \
                             Use ParRateMethod::ForwardBased for seasoned swaps.",
                            as_of, dates[0]
                        )
                    ));
                }

                let disc_dc = disc.day_count();
                let t_as_of = disc_dc
                    .year_fraction(base, as_of, finstack_core::dates::DayCountCtx::default())?;
                let df_as_of = disc.df(t_as_of);

                // Guard against near-zero discount factors for numerical stability
                if df_as_of.abs() < DF_EPSILON {
                    return Err(finstack_core::error::Error::Validation(format!(
                        "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
                         This may indicate extreme rate scenarios or very long time horizons.",
                        df_as_of, DF_EPSILON
                    )));
                }

                // Numerator: P(as_of,T0) - P(as_of,Tn)
                let t0 = disc_dc
                    .year_fraction(base, dates[0], finstack_core::dates::DayCountCtx::default())?;
                let tn = disc_dc
                    .year_fraction(
                        base,
                        *dates.last().expect("Dates should not be empty"),
                        finstack_core::dates::DayCountCtx::default(),
                    )?;

                let p0_abs = disc.df(t0);
                let pn_abs = disc.df(tn);
                // df_as_of already validated above, safe to divide
                let p0 = p0_abs / df_as_of;
                let pn = pn_abs / df_as_of;
                let num = p0 - pn;

                // Denominator: Sum alpha_i P(as_of,Ti) for future cashflows
                let mut den = 0.0;
                let mut prev = dates[0];
                for &d in &dates[1..] {
                    // Only include future cashflows
                    if d <= as_of {
                        prev = d;
                        continue;
                    }

                    let alpha = irs
                        .fixed
                        .dc
                        .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
                    let t_d = disc_dc
                        .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())?;
                    let p_abs = disc.df(t_d);
                    // df_as_of already validated above, safe to divide
                    let p = p_abs / df_as_of;
                    den += alpha * p;
                    prev = d;
                }
                if den == 0.0 {
                    return Ok(0.0);
                }
                Ok(num / den)
            }
        }
    }
}
