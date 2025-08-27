#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::dates::Frequency;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::{Discount, Forward};

use crate::pricing::df::df_on;
use crate::pricing::result::ValuationResult;
use crate::traits::Priceable;

#[derive(Clone, Copy, Debug)]
pub enum PayReceive { PayFixed, ReceiveFixed }

#[derive(Clone, Debug)]
pub struct FixedLegSpec {
    pub disc_id: &'static str,
    pub rate: F,
    pub freq: Frequency,
    pub dc: DayCount,
    pub calendar: Option<&'static str>,
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
    pub calendar: Option<&'static str>,
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
    fn schedule(&self, start: Date, end: Date, freq: Frequency) -> Vec<Date> {
        // Use core ScheduleBuilder without adjustment for MVP
        finstack_core::dates::ScheduleBuilder::new(start, end)
            .frequency(freq)
            .build_raw()
            .collect()
    }

    fn annuity(&self, disc: &dyn Discount) -> F {
        let base = disc.base_date();
        let sched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq);
        let mut a = 0.0;
        let mut prev = sched[0];
        for &d in &sched[1..] {
            let yf = self.fixed.dc.year_fraction(prev, d).unwrap_or(0.0);
            let df = df_on(disc, base, d, self.fixed.dc);
            a += yf * df;
            prev = d;
        }
        a
    }

    fn pv_fixed(&self, disc: &dyn Discount) -> Money {
        let base = disc.base_date();
        let sched = self.schedule(self.fixed.start, self.fixed.end, self.fixed.freq);
        let mut pv = Money::new(0.0, self.notional.currency());
        let mut prev = sched[0];
        for &d in &sched[1..] {
            let yf = self.fixed.dc.year_fraction(prev, d).unwrap_or(0.0);
            let df = df_on(disc, base, d, self.fixed.dc);
            let cash = self.notional * (self.fixed.rate * yf);
            pv = (pv + (cash * df)).expect("ccy");
            prev = d;
        }
        pv
    }

    fn pv_float(&self, disc: &dyn Discount, fwd: &dyn Forward) -> Money {
        let base_d = disc.base_date();
        // Assume same base date for forward curve in MVP
        let base_f = base_d;
        let sched = self.schedule(self.float.start, self.float.end, self.float.freq);
        let mut pv = Money::new(0.0, self.notional.currency());
        let mut prev = sched[0];
        for &d in &sched[1..] {
            let t1 = self.float.dc.year_fraction(base_f, prev).unwrap_or(0.0);
            let t2 = self.float.dc.year_fraction(base_f, d).unwrap_or(0.0);
            let yf = self.float.dc.year_fraction(prev, d).unwrap_or(0.0);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            let df = df_on(disc, base_d, d, self.float.dc);
            pv = (pv + (coupon * df)).expect("ccy");
            prev = d;
        }
        pv
    }

    fn par_rate(&self, disc: &dyn Discount, fwd: &dyn Forward) -> F {
        let float_pv = self.pv_float(disc, fwd).amount();
        let ann = self.annuity(disc);
        if ann == 0.0 { 0.0 } else { float_pv / self.notional.amount() / ann }
    }
}

impl Priceable for InterestRateSwap {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let disc = curves.discount(self.fixed.disc_id)?;
        let fwd = curves.forecast(self.float.fwd_id)?;

        let pv_fixed = self.pv_fixed(&*disc);
        let pv_float = self.pv_float(&*disc, &*fwd);
        let value = match self.side {
            PayReceive::PayFixed => (pv_float - pv_fixed).expect("ccy"),
            PayReceive::ReceiveFixed => (pv_fixed - pv_float).expect("ccy"),
        };

        let mut res = ValuationResult::stamped(self.id.clone(), as_of, value);
        res.measures.insert("annuity".to_string(), self.annuity(&*disc));
        res.measures.insert("par_rate".to_string(), self.par_rate(&*disc, &*fwd));
        Ok(res)
    }
}


