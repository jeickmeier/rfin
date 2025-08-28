#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::dates::{Frequency, BusinessDayConvention, StubKind};
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

use crate::cashflow::builder::{cf, FixedCouponSpec, FloatingCouponSpec as BuilderFloat, CouponType};
use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Priceable, CashflowProvider, DatedFlows};

#[derive(Clone, Copy, Debug)]
pub enum PayReceive { PayFixed, ReceiveFixed }

#[derive(Clone, Debug)]
pub struct FixedLegSpec {
    pub disc_id: &'static str,
    pub rate: F,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
    pub start: Date,
    pub end: Date,
}

#[derive(Clone, Debug)]
pub struct FloatLegSpec {
    pub disc_id: &'static str,
    pub fwd_id: &'static str,
    pub spread_bp: F,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
    pub start: Date,
    pub end: Date,
}

#[derive(Clone, Debug)]
pub struct InterestRateSwap {
    pub id: String,
    pub notional: Money,
    pub side: PayReceive,
    pub fixed: FixedLegSpec,
    pub float: FloatLegSpec,
}

impl InterestRateSwap {
    fn schedule(&self, start: Date, end: Date, freq: Frequency, bdc: BusinessDayConvention, cal_id: &Option<&'static str>, stub: StubKind) -> Vec<Date> {
        let builder = finstack_core::dates::ScheduleBuilder::new(start, end)
            .frequency(freq)
            .stub_rule(stub);
        if let Some(id) = cal_id {
            if let Some(cal) = calendar_by_id(id) {
                return builder.adjust_with(bdc, cal).build().collect();
            }
        }
        builder.build_raw().collect()
    }

    fn annuity(&self, disc: &dyn Discount) -> F {
        // Derived from builder flows for display-only; compute sum(yf*df)
        let base = disc.base_date();
        let sched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq, self.fixed.bdc, &self.fixed.calendar_id, self.fixed.stub);
        if sched.len() < 2 { return 0.0; }
        let mut acc = 0.0;
        let mut prev = sched[0];
        for &d in &sched[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, self.fixed.dc);
            let df = DiscountCurve::df_on(disc, base, d, self.fixed.dc);
            acc += yf * df;
            prev = d;
        }
        acc
    }

    fn pv_fixed(&self, disc: &dyn Discount) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let mut b = cf();
        b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id,
                stub: self.fixed.stub,
            });
        let sched = b.build()?;
        // Discount coupon flows only
        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter(|cf| cf.kind == crate::cashflow::primitives::CFKind::Fixed || cf.kind == crate::cashflow::primitives::CFKind::Stub)
            .map(|cf| (cf.date, cf.amount))
            .collect();
        flows.npv(disc, base, sched.day_count)
    }

    fn pv_float(&self, disc: &dyn Discount, fwd: &dyn Forward) -> finstack_core::Result<Money> {
        let base_d = disc.base_date();
        // Assume same base date for forward curve in MVP
        let base_f = base_d;
        let sched_dates = self.schedule(self.float.start, self.float.end, self.float.freq, self.float.bdc, &self.float.calendar_id, self.float.stub);

        // Build coupon flows from forward curve, then discount via generic npv
        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        let mut prev = sched_dates[0];
        let mut flows: Vec<(Date, Money)> = Vec::with_capacity(sched_dates.len().saturating_sub(1));
        for &d in &sched_dates[1..] {
            let t1 = DiscountCurve::year_fraction(base_f, prev, self.float.dc);
            let t2 = DiscountCurve::year_fraction(base_f, d, self.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, self.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            flows.push((d, coupon));
            prev = d;
        }
        flows.npv(disc, base_d, self.float.dc)
    }

    fn par_rate(&self, disc: &dyn Discount, fwd: &dyn Forward) -> F {
        let float_pv = self.pv_float(disc, fwd).map(|m| m.amount()).unwrap_or(0.0);
        let ann = self.annuity(disc);
        if ann == 0.0 { 0.0 } else { float_pv / self.notional.amount() / ann }
    }

    #[allow(dead_code)]
    fn pv_with_discount_bump(&self, disc: &dyn Discount, fwd: &dyn Forward, bp: F) -> (Money, Money) {
        // Parallel bump of discount zero rates by +/- bp
        let base_d = disc.base_date();
        let base_f = base_d;

        // Helper to compute adjusted df as df * exp(-bp * t)
        let adj_df = |date: Date, dc: DayCount| -> F {
            let t = DiscountCurve::year_fraction(base_d, date, dc);
            let df = DiscountCurve::df_on(disc, base_d, date, dc);
            df * (-bp * t).exp()
        };

        // Fixed leg
        let fsched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq, self.fixed.bdc, &self.fixed.calendar_id, self.fixed.stub);
        let mut pv_fixed = Money::new(0.0, self.notional.currency());
        let mut prev = fsched[0];
        for &d in &fsched[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, self.fixed.dc);
            let cash = self.notional * (self.fixed.rate * yf);
            let dfb = adj_df(d, self.fixed.dc);
            pv_fixed = (pv_fixed + (cash * dfb)).unwrap_or(pv_fixed);
            prev = d;
        }

        // Float leg
        let lsched = self.schedule(self.float.start, self.float.end, self.float.freq, self.float.bdc, &self.float.calendar_id, self.float.stub);
        let mut prevl = lsched[0];
        let mut pv_float = Money::new(0.0, self.notional.currency());
        for &d in &lsched[1..] {
            let t1 = DiscountCurve::year_fraction(base_f, prevl, self.float.dc);
            let t2 = DiscountCurve::year_fraction(base_f, d, self.float.dc);
            let yf = DiscountCurve::year_fraction(prevl, d, self.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            let dfb = adj_df(d, self.float.dc);
            pv_float = (pv_float + (coupon * dfb)).unwrap_or(pv_float);
            prevl = d;
        }
        (pv_fixed, pv_float)
    }
}

impl Priceable for InterestRateSwap {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let disc = curves.discount(self.fixed.disc_id)?;
        let fwd = curves.forecast(self.float.fwd_id)?;

        let pv_fixed = self.pv_fixed(&*disc)?;
        let pv_float = self.pv_float(&*disc, &*fwd)?;
        let value = match self.side {
            PayReceive::PayFixed => (pv_float - pv_fixed)?,
            PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
        };

        let mut res = ValuationResult::stamped(self.id.clone(), as_of, value);
        res.measures.insert("annuity".to_string(), self.annuity(&*disc));
        res.measures.insert("par_rate".to_string(), self.par_rate(&*disc, &*fwd));
        // DV01 (sign-adjusted annuity proxy): positive receive-fixed, negative pay-fixed
        let ann = self.annuity(&*disc);
        let dv01 = match self.side { PayReceive::ReceiveFixed => ann, PayReceive::PayFixed => -ann };
        res.measures.insert("dv01".to_string(), dv01);
        Ok(res)
    }
}

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Use builder to generate both legs; then map signs by side
        let mut fixed_b = cf();
        fixed_b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec { coupon_type: CouponType::Cash, rate: self.fixed.rate, freq: self.fixed.freq, dc: self.fixed.dc, bdc: self.fixed.bdc, calendar_id: self.fixed.calendar_id, stub: self.fixed.stub });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = cf();
        float_b.principal(self.notional, self.float.start, self.float.end)
            .floating_cf(BuilderFloat { index_id: self.float.fwd_id, margin_bp: self.float.spread_bp, gearing: 1.0, coupon_type: CouponType::Cash, freq: self.float.freq, dc: self.float.dc, bdc: self.float.bdc, calendar_id: self.float.calendar_id, stub: self.float.stub, reset_lag_days: 2 });
        let float_sched = float_b.build()?;

        let mut flows: Vec<(Date, Money)> = Vec::new();
        for cf in fixed_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed || cf.kind == crate::cashflow::primitives::CFKind::Stub {
                let amt = match self.side { PayReceive::ReceiveFixed => cf.amount, PayReceive::PayFixed => cf.amount * -1.0 };
                flows.push((cf.date, amt));
            }
        }
        for cf in float_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
                let c = cf.amount;
                let amt = match self.side { PayReceive::ReceiveFixed => c * -1.0, PayReceive::PayFixed => c };
                flows.push((cf.date, amt));
            }
        }
        Ok(flows)
    }
}


