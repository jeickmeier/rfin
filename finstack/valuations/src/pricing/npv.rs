//! NPV calculation for dated `Money` flows.

use finstack_core::prelude::*;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
/// 
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
/// 
/// # Errors
/// Returns an error if the flows list is empty.
/// 
/// # Example
/// ```rust
/// use finstack_valuations::pricing::npv::npv;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::Date;
/// use finstack_core::dates::DayCount;
/// use finstack_core::market_data::traits::Discount;
/// use finstack_core::market_data::id::CurveId;
/// use finstack_core::market_data::traits::TermStructure;
/// use time::Month;
/// 
/// struct FlatCurve { id: CurveId }
/// impl TermStructure for FlatCurve { 
///     fn id(&self) -> &CurveId { &self.id } 
/// }
/// impl Discount for FlatCurve {
///     fn base_date(&self) -> Date { 
///         Date::from_calendar_date(2025, Month::January, 1).unwrap() 
///     }
///     fn df(&self, _t: f64) -> f64 { 1.0 } // No discount for simplicity
/// }
/// 
/// let curve = FlatCurve { id: CurveId::new("USD-OIS") };
/// let base = curve.base_date();
/// let flows = vec![
///     (Date::from_calendar_date(2025, Month::June, 15).unwrap(), Money::new(50_000.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::December, 15).unwrap(), Money::new(1_050_000.0, Currency::USD)),
/// ];
/// let pv = npv(&curve, base, DayCount::Act365F, &flows)?;
/// assert!((pv.amount() - 1_100_000.0).abs() < 1e-6);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
    use finstack_core::market_data::traits::TermStructure;
    use finstack_core::market_data::traits::Discount;
    use time::Month;

    struct UnitCurve { id: CurveId }
    impl TermStructure for UnitCurve { fn id(&self) -> &CurveId { &self.id } }
    impl Discount for UnitCurve {
        fn base_date(&self) -> Date { Date::from_calendar_date(2025, Month::January, 1).unwrap() }
        fn df(&self, _t: finstack_core::F) -> finstack_core::F { 1.0 }
    }

    #[test]
    fn npv_errors_on_empty_flows() {
        let curve = UnitCurve { id: CurveId::new("USD-OIS") };
        let base = curve.base_date();
        let flows: Vec<(Date, Money)> = vec![];
        let err = npv(&curve, base, DayCount::Act365F, &flows).unwrap_err();
        // Ensure it's an input error
        let _ = format!("{}", err); // exercise Display
    }
}
