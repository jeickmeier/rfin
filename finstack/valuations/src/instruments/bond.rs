#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;

use crate::pricing::df::df_on;
use crate::pricing::quotes::accrued_interest;
use crate::pricing::result::ValuationResult;
use crate::traits::Priceable;

#[derive(Clone, Debug)]
pub struct Bond {
    pub id: String,
    pub notional: Money,
    pub coupon: F,
    pub freq: finstack_core::dates::Frequency,
    pub dc: DayCount,
    pub issue: Date,
    pub maturity: Date,
    pub disc_id: &'static str,
}

impl Bond {
    fn schedule(&self) -> Vec<Date> {
        finstack_core::dates::ScheduleBuilder::new(self.issue, self.maturity)
            .frequency(self.freq)
            .build_raw()
            .collect()
    }

    fn pv(&self, disc: &dyn Discount) -> Money {
        let sched = self.schedule();
        let base = disc.base_date();
        let mut pv = Money::new(0.0, self.notional.currency());
        let mut prev = sched[0];
        for &d in &sched[1..] {
            let yf = self.dc.year_fraction(prev, d).unwrap_or(0.0);
            let df = df_on(disc, base, d, self.dc);
            let cpn = self.notional * (self.coupon * yf);
            pv = (pv + (cpn * df)).expect("ccy");
            prev = d;
        }
        // Redemption at maturity
        let df_mat = df_on(disc, base, self.maturity, self.dc);
        pv = (pv + (self.notional * df_mat)).expect("ccy");
        pv
    }
}

impl Priceable for Bond {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let disc = curves.discount(self.disc_id)?;
        let value = self.pv(&*disc);

        // Accrued interest between last and next coupon around as_of
        let sched = self.schedule();
        let (mut last, mut next) = (self.issue, self.maturity);
        for w in sched.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a <= as_of && as_of < b {
                last = a;
                next = b;
                break;
            }
        }
        let ai = accrued_interest(self.notional, self.coupon, last, as_of, next, self.dc);

        let mut res = ValuationResult::stamped(self.id.clone(), as_of, value);
        res.measures.insert("accrued".to_string(), ai.amount());
        Ok(res)
    }
}


