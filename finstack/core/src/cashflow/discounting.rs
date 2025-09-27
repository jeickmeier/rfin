//! Helpers for discounting dated cashflows against market curves.

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::market_data::traits::Discounting;
use crate::money::Money;

/// Objects that can be present-valued against a `Discount` curve.
///
/// Provides a unified interface for NPV calculations across different
/// cashflow representations and instrument types.
pub trait Discountable {
    /// Output type for the NPV calculation.
    type PVOutput;

    /// Compute present value using the given discount curve and day count.
    fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> Self::PVOutput;
}

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount` with static dispatch.
///
/// This generic helper avoids dynamic dispatch on the discount curve in tight loops.
pub fn npv_static<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    if flows.is_empty() {
        return Err(crate::error::InputError::TooFewPoints.into());
    }
    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);
    for (d, amt) in flows {
        let t = if *d == base {
            0.0
        } else {
            dc.year_fraction(base, *d, DayCountCtx::default())
                .unwrap_or(0.0)
        };
        let df = disc.df(t);
        let disc_amt = *amt * df;
        total = (total + disc_amt)?;
    }
    Ok(total)
}

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
///
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
fn npv(
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    npv_static(disc, base, dc, flows)
}

impl Discountable for &[(Date, Money)] {
    type PVOutput = crate::Result<Money>;

    fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> crate::Result<Money> {
        npv(disc, base, dc, self)
    }
}

impl Discountable for Vec<(Date, Money)> {
    type PVOutput = crate::Result<Money>;

    fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> crate::Result<Money> {
        npv(disc, base, dc, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::traits::TermStructure;
    use crate::types::CurveId;
    use crate::F;
    use time::Month;

    struct FlatCurve {
        id: CurveId,
    }

    impl TermStructure for FlatCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for FlatCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        }
        fn df(&self, _t: F) -> F {
            1.0
        }
    }

    #[test]
    fn tuples_discountable_paths_through() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let flows = vec![
            (base, Money::new(10.0, crate::currency::Currency::USD)),
            (base, Money::new(5.0, crate::currency::Currency::USD)),
        ];
        let pv = flows.npv(&curve, base, DayCount::Act365F).unwrap();
        assert!((pv.amount() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn npv_errors_on_empty_flows() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let flows: Vec<(Date, Money)> = vec![];
        let err = super::npv(&curve, base, DayCount::Act365F, &flows).unwrap_err();
        let _ = format!("{}", err);
    }
}
