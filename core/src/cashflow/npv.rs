use crate::cashflow::leg::CashFlowLeg;
use crate::cashflow::primitives::CashFlow;
use crate::dates::Date;
use crate::money::Money;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Minimal discount curve trait for present-value calculations.
pub trait DiscountCurve: Sync {
    /// Discount factor for the given date (1.0 = today).
    fn df(&self, date: Date) -> f64;
}

/// Generic trait for present-valueable objects.
pub trait Discountable {
    type PVOutput;
    fn npv<C: DiscountCurve>(&self, curve: &C) -> Self::PVOutput;
}

impl Discountable for CashFlowLeg {
    type PVOutput = Money;

    fn npv<C: DiscountCurve>(&self, curve: &C) -> Money {
        #[cfg(feature = "parallel")]
        {
            let currency = self.notional.currency();
            let total = self
                .flows
                .par_iter()
                .map(|cf| {
                    let df = curve.df(cf.date);
                    cf.amount.amount() * df
                })
                .sum::<f64>();
            Money::new(total, currency)
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut pv = Money::new(0.0, self.notional.currency());
            for cf in &self.flows {
                let df = curve.df(cf.date);
                let disc = cf.amount * df;
                pv = (pv + disc).expect("currency mismatch");
            }
            pv
        }
    }
}

impl Discountable for [CashFlow] {
    type PVOutput = Money;

    fn npv<C: DiscountCurve>(&self, curve: &C) -> Money {
        if self.is_empty() {
            use crate::currency::Currency;
            return Money::new(0.0, Currency::USD);
        }
        let currency = self[0].amount.currency();

        #[cfg(feature = "parallel")]
        {
            let total = self
                .par_iter()
                .map(|cf| cf.amount.amount() * curve.df(cf.date))
                .sum::<f64>();
            Money::new(total, currency)
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut pv = Money::new(0.0, currency);
            for cf in self {
                let df = curve.df(cf.date);
                let disc = cf.amount * df;
                pv = (pv + disc).expect("currency mismatch");
            }
            pv
        }
    }
}

/// Convenience helper: sum PV across multiple legs/arrays.
pub fn npv_portfolio<C, T>(legs: &[T], curve: &C) -> Money
where
    C: DiscountCurve,
    T: Discountable<PVOutput = Money> + Sync,
{
    use crate::currency::Currency;
    if legs.is_empty() {
        return Money::new(0.0, Currency::USD);
    }
    let currency = legs[0].npv(curve).currency();

    #[cfg(feature = "parallel")]
    {
        let total = legs
            .par_iter()
            .map(|leg| leg.npv(curve).amount())
            .sum::<f64>();
        Money::new(total, currency)
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut total = Money::new(0.0, currency);
        for leg in legs {
            total = (total + leg.npv(curve)).expect("currency mismatch");
        }
        total
    }
}
