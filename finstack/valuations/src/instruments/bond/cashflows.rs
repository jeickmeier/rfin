//! Cashflow construction for bonds (deterministic schedules only).

use finstack_core::dates::adjust;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Date, DayCountCtx, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use time::Duration;

use crate::cashflow::builder::{CashFlowSchedule, FixedCouponSpec};
use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::{CashflowProvider, DatedFlows};

use super::types::Bond;

impl CashflowProvider for Bond {
    fn build_schedule(&self, _curves: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        if let Some(ref custom) = self.custom_cashflows {
            let flows: Vec<(Date, Money)> = custom
                .flows
                .iter()
                .filter_map(|cf| match cf.kind {
                    CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                    CFKind::Amortization => Some((
                        cf.date,
                        Money::new(-cf.amount.amount(), cf.amount.currency()),
                    )),
                    CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                    _ => None,
                })
                .collect();
            return Ok(flows);
        }

        // Floating-rate path: compute coupons off forward index, and source amortization via builder
        if let Some(ref fl) = self.float {
            use crate::cashflow::builder::schedule_utils::build_dates as build_periods;

            let fwd = _curves.get_forward_ref(fl.fwd_id.clone())?;

            // 1) Build amortization-only schedule using builder (dedup amort logic)
            let mut b_am = CashFlowSchedule::builder();
            b_am.principal(self.notional, self.issue, self.maturity);
            if let Some(am) = &self.amortization {
                b_am.amortization(am.clone());
            }
            let amort_sched = b_am.build()?;

            // Map amortization and redemption flows from builder schedule
            let mut flows: Vec<(Date, Money)> = amort_sched
                .flows
                .iter()
                .filter_map(|cf| match cf.kind {
                    CFKind::Amortization => Some((
                        cf.date,
                        Money::new(-cf.amount.amount(), cf.amount.currency()),
                    )),
                    CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                    _ => None,
                })
                .collect();

            // 2) Build coupon period schedule using instrument-level conventions
            let schedule = build_periods(
                self.issue,
                self.maturity,
                self.freq,
                self.stub,
                self.bdc,
                self.calendar_id.as_deref(),
            );
            let periods = schedule.dates;
            if periods.len() < 2 {
                return Ok(flows);
            }

            // Prepare outstanding at end of each unique date from amortization schedule
            let out_by_date = amort_sched.outstanding_by_date();
            let mut out_map: hashbrown::HashMap<Date, f64> =
                hashbrown::HashMap::with_capacity(out_by_date.len());
            for (d, m) in out_by_date {
                out_map.insert(d, m.amount());
            }

            // 3) Compute coupons using outstanding BEFORE amortization on the period end (use previous date's outstanding)
            let f_dc = fwd.day_count();
            let ccy = self.notional.currency();
            for w in periods.windows(2) {
                let start = w[0];
                let end = w[1];

                // Reset date adjusted by reset lag and calendar
                let mut reset_date = start - Duration::days(fl.reset_lag_days as i64);
                if let Some(id) = &self.calendar_id {
                    if let Some(cal) = calendar_by_id(id) {
                        reset_date = adjust(reset_date, self.bdc, cal)?;
                    }
                }

                let t_reset = f_dc
                    .year_fraction(fwd.base_date(), reset_date, DayCountCtx::default())
                    .unwrap_or(0.0);
                let yf = self
                    .dc
                    .year_fraction(start, end, DayCountCtx::default())
                    .unwrap_or(0.0);
                if yf > 0.0 {
                    // Outstanding base is outstanding at the end of previous date (start)
                    let base_out = *out_map.get(&start).unwrap_or(&self.notional.amount());
                    let f = fwd.rate(t_reset);
                    let rate = fl.gearing * f + fl.margin_bp * 1e-4;
                    let coupon = base_out * rate * yf;
                    if coupon != 0.0 {
                        flows.push((end, Money::new(coupon, ccy)));
                    }
                }
            }

            // 4) Return combined flows (coupons + amortization + redemption)
            flows.sort_by_key(|(d, _)| *d);
            return Ok(flows);
        }

        let mut b = CashFlowSchedule::builder();
        b.principal(self.notional, self.issue, self.maturity);
        if let Some(am) = &self.amortization {
            b.amortization(am.clone());
        }
        b.fixed_cf(FixedCouponSpec {
            coupon_type: crate::cashflow::builder::CouponType::Cash,
            rate: self.coupon,
            freq: self.freq,
            dc: self.dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        });
        let sched = b.build()?;

        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                CFKind::Amortization => Some((
                    cf.date,
                    Money::new(-cf.amount.amount(), cf.amount.currency()),
                )),
                CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                _ => None,
            })
            .collect();

        Ok(flows)
    }

    /// Build full cashflow schedule with CFKind metadata for precise classification.
    ///
    /// This leverages Bond's existing `get_full_schedule()` method to provide
    /// complete cashflow information including CFKind classification and
    /// outstanding balance tracking.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Use Bond's existing get_full_schedule method for precise CFKind classification
        self.get_full_schedule(curves)
    }
}
