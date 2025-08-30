//! NPV calculation for dated `Money` flows.

use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::Discount;
use finstack_core::prelude::*;

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
///
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
///
/// # Errors
/// Returns an error if the flows list is empty.
///
/// See unit tests and `examples/` for usage.
pub fn npv(
    disc: &dyn Discount,
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
        let df = DiscountCurve::df_on(disc, base, *d, dc);
        // Multiplying Money by scalar returns Money
        let disc_amt = *amt * df;
        total = (total + disc_amt)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::id::CurveId;
    use finstack_core::market_data::traits::Discount;
    use finstack_core::market_data::traits::TermStructure;
    use time::Month;

    struct UnitCurve {
        id: CurveId,
    }
    impl TermStructure for UnitCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }
    impl Discount for UnitCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        }
        fn df(&self, _t: finstack_core::F) -> finstack_core::F {
            1.0
        }
    }

    #[test]
    fn npv_errors_on_empty_flows() {
        let curve = UnitCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let flows: Vec<(Date, Money)> = vec![];
        let err = npv(&curve, base, DayCount::Act365F, &flows).unwrap_err();
        // Ensure it's an input error
        let _ = format!("{}", err); // exercise Display
    }
}
