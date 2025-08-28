#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{CashflowProvider, Priceable, DatedFlows};
use crate::cashflow::builder::{cf, FixedCouponSpec, CouponType};
use finstack_core::dates::{BusinessDayConvention, StubKind, Frequency};

#[derive(Clone, Debug)]
pub struct Deposit {
    pub id: String,
    pub notional: Money,
    pub start: Date,
    pub end: Date,
    pub day_count: DayCount,
    /// Optional quoted simple rate r (annualised) for the deposit.
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: &'static str,
}

impl Deposit {
    /// Year fraction of the deposit.
    fn yf(&self) -> F {
        DiscountCurve::year_fraction(self.start, self.end, self.day_count)
    }

    /// Compute par (simple) rate from curves.
    fn par_rate(&self, disc: &dyn Discount) -> F {
        // r_par = (DF(start)/DF(end) - 1) / yf
        let base = disc.base_date();
        let df_s = DiscountCurve::df_on(disc, base, self.start, self.day_count);
        let df_e = DiscountCurve::df_on(disc, base, self.end, self.day_count);
        let yf = self.yf();
        if yf == 0.0 { return 0.0; }
        (df_s / df_e - 1.0) / yf
    }

    /// Compute implied DF(end) from a quoted simple rate.
    fn df_end_from_quote(&self, disc: &dyn Discount, r: F) -> F {
        // DF(end) = DF(start) / (1 + r * yf)
        let base = disc.base_date();
        let df_s = DiscountCurve::df_on(disc, base, self.start, self.day_count);
        let yf = self.yf();
        df_s / (1.0 + r * yf)
    }
}

impl CashflowProvider for Deposit {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Build a single-period schedule using a custom day-step equal to the total span
        let days = (self.end - self.start).whole_days();
        let rate = self.quote_rate.unwrap_or(0.0);

        let mut b = cf();
        b.principal(self.notional, self.start, self.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate,
                freq: if days <= 1 { Frequency::daily() } else if days == 7 { Frequency::weekly() } else if days == 14 { Frequency::biweekly() } else { Frequency::monthly() },
                dc: self.day_count,
                bdc: BusinessDayConvention::Unadjusted,
                calendar_id: None,
                stub: StubKind::None,
            });
        let sched = b.build()?;

        // Map to two-flow holder schedule: principal out at start; redemption at end including interest
        // Sum all amounts on end date except the initial notional outflow
        let mut redemption = Money::new(0.0, self.notional.currency());
        for cf in &sched.flows {
            if cf.date == self.end {
                // Include both coupon and final notional
                redemption = (redemption + cf.amount)?;
            }
        }
        Ok(vec![(self.start, self.notional * -1.0), (self.end, redemption)])
    }
}

impl Priceable for Deposit {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let disc = curves.discount(self.disc_id)?;
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        let value = flows.npv(&*disc, base, self.day_count)?;

        let mut res = ValuationResult::stamped(self.id.clone(), as_of, value);
        // Measures useful for bootstrapping
        res.measures.insert("yf".to_string(), self.yf());
        // For transparency, keep DF measures from curves
        let df_s = DiscountCurve::df_on(&*disc, base, self.start, self.day_count);
        let df_e = DiscountCurve::df_on(&*disc, base, self.end, self.day_count);
        res.measures.insert("df_start".to_string(), df_s);
        res.measures.insert("df_end".to_string(), df_e);
        res.measures.insert("par_rate".to_string(), self.par_rate(&*disc));
        if let Some(r) = self.quote_rate {
            res.measures.insert("df_end_from_quote".to_string(), self.df_end_from_quote(&*disc, r));
            res.measures.insert("quote_rate".to_string(), r);
        }
        Ok(res)
    }
}


