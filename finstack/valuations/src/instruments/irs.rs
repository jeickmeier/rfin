#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::dates::{Frequency, BusinessDayConvention, StubKind};
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

use crate::pricing::legs;
// use crate::cashflow::leg::CashFlowLeg; // not needed directly here
use crate::cashflow::notional::Notional;
use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Priceable, CashflowProvider, DatedFlows};
use crate::cashflow::leg::CashFlowLeg;

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
        let base = disc.base_date();
        let sched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq, self.fixed.bdc, &self.fixed.calendar_id, self.fixed.stub);
        legs::annuity(disc, base, self.fixed.dc, &sched)
    }

    fn pv_fixed(&self, disc: &dyn Discount) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let sched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq, self.fixed.bdc, &self.fixed.calendar_id, self.fixed.stub);
        let leg = CashFlowLeg::fixed_rate(
            Notional::par(self.notional.amount(), self.notional.currency()),
            self.fixed.rate,
            sched.iter().copied(),
            self.fixed.dc,
        )?;
        leg.npv(disc, base, self.fixed.dc)
    }

    fn pv_float(&self, disc: &dyn Discount, fwd: &dyn Forward) -> finstack_core::Result<Money> {
        let base_d = disc.base_date();
        // Assume same base date for forward curve in MVP
        let base_f = base_d;
        let sched = self.schedule(self.float.start, self.float.end, self.float.freq, self.float.bdc, &self.float.calendar_id, self.float.stub);
        // Build spread-only leg for transparency (flows not used for PV below, but available via build_schedule)
        let _spread_leg = CashFlowLeg::floating_spread(
            self.notional,
            self.float.spread_bp,
            1.0,
            0,
            sched.iter().copied(),
            self.float.dc,
        )?;

        // PV via forward curve (gearing assumed 1.0 here)
        let mut pv = Money::new(0.0, self.notional.currency());
        if !sched.is_empty() {
            let mut prev = sched[0];
            for &d in &sched[1..] {
                let t1 = DiscountCurve::year_fraction(base_f, prev, self.float.dc);
                let t2 = DiscountCurve::year_fraction(base_f, d, self.float.dc);
                let yf = DiscountCurve::year_fraction(prev, d, self.float.dc);
                let f = fwd.rate_period(t1, t2);
                let rate = f + (self.float.spread_bp * 1e-4);
                let coupon = self.notional * (rate * yf);
                let df = DiscountCurve::df_on(disc, base_d, d, self.float.dc);
                pv = (pv + (coupon * df))?;
                prev = d;
            }
        }
        Ok(pv)
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
        let mut flows: Vec<(Date, Money)> = Vec::new();
        // Fixed leg flows
        let fs = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq, self.fixed.bdc, &self.fixed.calendar_id, self.fixed.stub);
        let mut prev = fs[0];
        for &d in &fs[1..] {
            let yf = DiscountCurve::year_fraction(prev, d, self.fixed.dc);
            let c = self.notional * (self.fixed.rate * yf);
            // Receive-fixed adds positive, pay-fixed negative
            let amt = match self.side { PayReceive::ReceiveFixed => c, PayReceive::PayFixed => c * -1.0 };
            flows.push((d, amt));
            prev = d;
        }
        // Float leg flows as projected coupons (sign opposite side)
        let ls = self.schedule(self.float.start, self.float.end, self.float.freq, self.float.bdc, &self.float.calendar_id, self.float.stub);
        let mut prevl = ls[0];
        for &d in &ls[1..] {
            let yf = DiscountCurve::year_fraction(prevl, d, self.float.dc);
            // Use spread only for generic flows; forward component belongs to curve-linked PV
            let c = self.notional * ((self.float.spread_bp * 1e-4) * yf);
            let amt = match self.side { PayReceive::ReceiveFixed => c * -1.0, PayReceive::PayFixed => c };
            flows.push((d, amt));
            prevl = d;
        }
        Ok(flows)
    }
}


