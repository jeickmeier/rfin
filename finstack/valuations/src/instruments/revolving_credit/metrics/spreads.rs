//! Spread metrics for the Revolving Credit Facility.

use crate::instruments::revolving_credit::RevolvingCreditFacility;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use crate::cashflow::traits::CashflowProvider;
use finstack_core::Result;
use crate::instruments::revolving_credit::types::ResetConvention;
use crate::instruments::common::traits::Instrument;

pub struct RcfISpreadCalculator;
pub struct RcfZSpreadCalculator;

impl MetricCalculator for RcfISpreadCalculator {
    fn dependencies(&self) -> &[MetricId] { &[] }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        use crate::cashflow::builder::schedule_utils::build_dates_with_eom;
        use finstack_core::dates::DayCountCtx;

        let rcf: &RevolvingCreditFacility = context.instrument_as()?;

        // Only defined for floating-rate RCF
        let (fwd_id, orig_spread_dec, reset_convention) = match &rcf.interest {
            crate::instruments::revolving_credit::InterestRateSpec::Floating { fwd_id, spread_bp, reset_convention, .. } => (fwd_id, *spread_bp / 10_000.0, *reset_convention),
            _ => return Ok(0.0),
        };

        let disc = context.curves.get_discount_ref(&rcf.disc_id)?;
        let fwd = context.curves.get_forward_ref(fwd_id.as_ref())?;
        let base_date = disc.base_date();
        let as_of = context.as_of;

        // Build period schedule over which interest is paid
        let sched = build_dates_with_eom(
            rcf.start_date,
            rcf.maturity_date,
            rcf.payment_frequency,
            rcf.payment_stub,
            rcf.payment_bdc,
            rcf.payment_calendar_id.as_deref(),
            rcf.align_end_of_month,
        );
        if sched.dates.len() < 2 {
            return Ok(0.0);
        }

        // Fetch existing detailed flows to get floating interest amounts
        let full = rcf.build_full_schedule(&context.curves, as_of)?;

        // Precompute discount scaling from as_of
        let dc = disc.day_count();
        let t_as_of = dc
            .year_fraction(base_date, as_of, DayCountCtx::default())
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        // Denominator: sum (accrual_base_i * DF(as_of, pay_i))
        // Numerator: target_pv - PV_noninterest - sum (fwd_i * accrual_base_i * DF)
        let mut denom = 0.0;
        let mut sum_fwd_part = 0.0;
        let mut pv_non_interest = 0.0;

        // Helper to discount a cashflow amount at date
        let df_on = |d: finstack_core::dates::Date| -> f64 {
            let t = dc
                .year_fraction(base_date, d, DayCountCtx::default())
                .unwrap_or(0.0);
            let df_abs = disc.df(t);
            if df_as_of != 0.0 { df_abs / df_as_of } else { 1.0 }
        };

        // Index over periods to find matching floating interest cashflow
        let fwd_dc = fwd.day_count();
        for w in sched.dates.windows(2) {
            let end = w[1];
            if end <= as_of { continue; }

            // Find floating interest cashflow at this pay date
            let maybe_cf = full.flows.iter().find(|cf| cf.date == end && cf.kind == finstack_core::cashflow::primitives::CFKind::FloatReset);
            if let Some(cf) = maybe_cf {
                // Compute forward over the fixing/payment window per reset convention
                let (t_fix, t_pay) = match reset_convention {
                    ResetConvention::InAdvance => {
                        let t_pay = fwd_dc.year_fraction(fwd.base_date(), end, DayCountCtx::default()).unwrap_or(0.0);
                        let t_fix = (t_pay - fwd.tenor()).max(0.0);
                        (t_fix, t_pay)
                    }
                    ResetConvention::InArrears => {
                        let t_pay = fwd_dc.year_fraction(fwd.base_date(), end, DayCountCtx::default()).unwrap_or(0.0);
                        let t_fix = (t_pay - fwd.tenor()).max(0.0);
                        (t_fix, t_pay)
                    }
                    ResetConvention::LagDays => {
                        // Approximate with arrears window for simplicity here
                        let t_pay = fwd_dc.year_fraction(fwd.base_date(), end, DayCountCtx::default()).unwrap_or(0.0);
                        let t_fix = (t_pay - fwd.tenor()).max(0.0);
                        (t_fix, t_pay)
                    }
                };
                let fwd_rate = if t_pay > t_fix { fwd.rate_period(t_fix, t_pay) } else { 0.0 };

                // Effective accrual base = interest_amount / (fwd + orig_spread)
                let rate_base = fwd_rate + orig_spread_dec;
                if rate_base != 0.0 {
                    let accrual_base = cf.amount.amount() / rate_base;
                    let df = df_on(end);
                    denom += accrual_base * df;
                    sum_fwd_part += accrual_base * fwd_rate * df;
                }
            }
        }

        // PV of all non-floating-interest flows (fees, principal, fixed coupons), discounted from as_of
        for cf in &full.flows {
            if cf.date <= as_of { continue; }
            if cf.kind != finstack_core::cashflow::primitives::CFKind::FloatReset {
                pv_non_interest += cf.amount.amount() * df_on(cf.date);
            }
        }

        // Target PV is current instrument PV
        let target_pv = rcf.value(&context.curves, as_of)?.amount();
        if denom.abs() < f64::EPSILON {
            return Ok(0.0);
        }
        // Solve: target = PV_non_int + sum(accrual*(fwd + s)*df)
        let s = (target_pv - pv_non_interest - sum_fwd_part) / denom;
        Ok(s)
    }
}

impl MetricCalculator for RcfZSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] { &[] }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        use crate::instruments::structured_credit::calculate_tranche_z_spread as zsolve;
        let rcf: &RevolvingCreditFacility = context.instrument_as()?;
        let flows = RevolvingCreditFacility::build_schedule(rcf, &context.curves, context.as_of)?;
        let disc = context.curves.get_discount_ref(&rcf.disc_id)?;
        let target = finstack_core::money::Money::new(0.0, rcf.credit_limit.currency());
        let z = zsolve(&flows, disc, target, context.as_of)?;
        Ok(z)
    }
}

