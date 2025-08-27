#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::Discount;

use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use crate::pricing::legs;
use crate::pricing::quotes::{accrued_interest, bond_dirty_from_ytm, bond_ytm_from_dirty, bond_duration_mac_mod};
use crate::pricing::quotes::{bond_ytm_from_dirty_with_redemption};
use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Priceable, CashflowProvider, DatedFlows};
// Intentionally do not import AmortizationSpec here; we re-export it below
use crate::cashflow::leg::CashFlowLeg;
use crate::cashflow::notional::Notional;
use crate::cashflow::primitives::CFKind;

// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use crate::cashflow::amortization::AmortizationSpec;

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
    /// Optional quoted clean price (per notional unit). If provided, we compute YTM measures.
    pub quoted_clean: Option<F>,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional amortization specification (principal paid during life).
    pub amortization: Option<AmortizationSpec>,
}

#[derive(Clone, Debug)]
pub struct CallPut { pub date: Date, pub price_pct_of_par: F }

#[derive(Clone, Debug, Default)]
pub struct CallPutSchedule { pub calls: Vec<CallPut>, pub puts: Vec<CallPut> }

// Removed local duplicate; using cashflow::amortization::AmortizationSpec

impl Bond {
    fn schedule(&self) -> Vec<Date> {
        finstack_core::dates::ScheduleBuilder::new(self.issue, self.maturity)
            .frequency(self.freq)
            .build_raw()
            .collect()
    }

    fn pv(&self, disc: &dyn Discount, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(disc, base, self.dc)
    }
}

impl Priceable for Bond {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        let disc = curves.discount(self.disc_id)?;
        let value = self.pv(&*disc, curves, as_of)?;

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

        // If we have a quoted clean price, compute YTM and duration measures.
        if let Some(clean_px) = self.quoted_clean {
            // Dirty = clean + accrued (per amount); construct Money for dirty
            let dirty_amt = clean_px + ai.amount();
            let dirty = Money::new(dirty_amt, self.notional.currency());
            let sched = self.schedule();
            let ytm = bond_ytm_from_dirty(self.notional, self.coupon, &sched, self.dc, as_of, dirty);
            let (d_mac, d_mod) = bond_duration_mac_mod(self.notional, self.coupon, &sched, self.dc, as_of, ytm);
            let convex = crate::pricing::quotes::bond_convexity_numeric(self.notional, self.coupon, &sched, self.dc, as_of, ytm, 1e-4);
            res.measures.insert("ytm".to_string(), ytm);
            res.measures.insert("duration_mac".to_string(), d_mac);
            res.measures.insert("duration_mod".to_string(), d_mod);
            res.measures.insert("convexity".to_string(), convex);
            // Echo derived clean/dirty for convenience
            let recomputed_dirty = bond_dirty_from_ytm(self.notional, self.coupon, &sched, self.dc, as_of, ytm).map(|m| m.amount()).unwrap_or(0.0);
            res.measures.insert("price_dirty".to_string(), recomputed_dirty);
            res.measures.insert("price_clean".to_string(), recomputed_dirty - ai.amount());
        }

        // Yield-to-worst if a call/put schedule is provided
        if let Some(cp) = &self.call_put {
            let sched = self.schedule();
            // Filter candidate exercise dates >= as_of and within schedule range
            let mut candidates: Vec<(Date, Money)> = Vec::new();
            for c in &cp.calls {
                if c.date >= as_of && c.date <= self.maturity {
                    let redemption = self.notional * (c.price_pct_of_par / 100.0);
                    candidates.push((c.date, redemption));
                }
            }
            for p in &cp.puts {
                if p.date >= as_of && p.date <= self.maturity {
                    let redemption = self.notional * (p.price_pct_of_par / 100.0);
                    candidates.push((p.date, redemption));
                }
            }
            // Always include maturity redemption at 100%
            candidates.push((self.maturity, self.notional));

            // Compute dirty price implied by current discounting
            let base = disc.base_date();
            // PV of coupons via helper + maturity redemption
            let mut dirty_now = legs::pv_fixed_leg(&*disc, base, self.dc, self.notional, self.coupon, &sched)?;
            let df_mat = DiscountCurve::df_on(&*disc, base, self.maturity, self.dc);
            dirty_now = (dirty_now + (self.notional * df_mat))?;

            // Choose worst (minimum) yield, tie-breaker earliest date
            let mut best_ytm = f64::INFINITY;
            let mut best_date = self.maturity;
            for (exercise, red) in candidates {
                // Truncate schedule to exercise date
                let mut trunc: Vec<Date> = sched.iter().cloned().filter(|d| *d <= exercise).collect();
                if *trunc.last().unwrap() != exercise { trunc.push(exercise); }
                let y = bond_ytm_from_dirty_with_redemption(self.notional, self.coupon, &trunc, self.dc, as_of, dirty_now, red);
                if y < best_ytm - 1e-12 || ((y - best_ytm).abs() <= 1e-12 && exercise < best_date) {
                    best_ytm = y;
                    best_date = exercise;
                }
            }
            res.measures.insert("ytw".to_string(), best_ytm);
            res.measures.insert("ytw_exercise_ts".to_string(), DiscountCurve::year_fraction(as_of, best_date, self.dc));
        }
        Ok(res)
    }
}

impl CashflowProvider for Bond {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        let schedule = self.schedule();
        let amort = self.amortization.clone().unwrap_or(AmortizationSpec::None);
        let leg = CashFlowLeg::fixed_rate(
            Notional { initial: self.notional, amort },
            self.coupon,
            schedule.iter().copied(),
            self.dc,
        )?;

        // Map to holder flows: coupons positive; amortization principal as positive inflow
        let mut flows: Vec<(Date, Money)> = leg
            .flows
            .iter()
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                CFKind::Amortization => Some((cf.date, Money::new(-cf.amount.amount(), cf.amount.currency()))),
                _ => None,
            })
            .collect();

        // Final redemption for remaining outstanding principal
        let paid_principal = leg
            .flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Amortization)
            .fold(0.0, |acc, cf| acc + (-cf.amount.amount()).max(0.0));
        let remaining = (self.notional.amount() - paid_principal).max(0.0);
        if remaining > 0.0 {
            flows.push((self.maturity, Money::new(remaining, self.notional.currency())));
        }
        Ok(flows)
    }
}


