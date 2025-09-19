//! Interface for objects that can be present-valued against a `Discount` curve.

use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::traits::Discounting;
use finstack_core::prelude::*;

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

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
///
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
///
/// # Errors
/// Returns an error if the flows list is empty.
///
/// See unit tests and `examples/` for usage.
fn npv(
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> finstack_core::Result<Money> {
    if flows.is_empty() {
        return Err(finstack_core::error::InputError::TooFewPoints.into());
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

impl Discountable for &[(Date, Money)] {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        npv(disc, base, dc, self)
    }
}

impl Discountable for Vec<(Date, Money)> {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        npv(disc, base, dc, self)
    }
}

impl Discountable for crate::cashflow::builder::CashFlowSchedule {
    type PVOutput = finstack_core::Result<Money>;

    /// Compute NPV of the cashflow schedule.
    ///
    /// Extracts date-amount pairs from the schedule and computes
    /// present value using the provided discount curve.
    ///
    /// See unit tests and `examples/` for usage.
    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        npv(disc, base, dc, &flows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::types::CurveId;
    use finstack_core::F;
    use time::Month;

    struct FlatCurve {
        id: CurveId,
    }
    impl finstack_core::market_data::traits::TermStructure for FlatCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }
    // No TermStructure impl needed in this test context
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
            (base, Money::new(10.0, Currency::USD)),
            (base, Money::new(5.0, Currency::USD)),
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
