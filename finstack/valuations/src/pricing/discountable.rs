#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::market_data::traits::Discount;

/// Objects that can be present-valued against a `Discount` curve.
pub trait Discountable {
    type PVOutput;
    fn npv(&self, disc: &dyn Discount, base: Date, dc: DayCount) -> Self::PVOutput;
}

impl Discountable for &[(Date, Money)] {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(&self, disc: &dyn Discount, base: Date, dc: DayCount) -> finstack_core::Result<Money> {
        super::npv::npv(disc, base, dc, self)
    }
}

impl Discountable for Vec<(Date, Money)> {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(&self, disc: &dyn Discount, base: Date, dc: DayCount) -> finstack_core::Result<Money> {
        super::npv::npv(disc, base, dc, self)
    }
}

impl Discountable for crate::cashflow::builder::CashFlowSchedule {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(&self, disc: &dyn Discount, base: Date, dc: DayCount) -> finstack_core::Result<Money> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        super::npv::npv(disc, base, dc, &flows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::id::CurveId;
    use finstack_core::market_data::traits::TermStructure;
    use finstack_core::F;
    use time::Month;

    struct FlatCurve { id: CurveId }
    impl TermStructure for FlatCurve { fn id(&self) -> &CurveId { &self.id } }
    impl Discount for FlatCurve {
        fn base_date(&self) -> Date { Date::from_calendar_date(2025, Month::January, 1).unwrap() }
        fn df(&self, _t: F) -> F { 1.0 }
    }

    #[test]
    fn tuples_discountable_paths_through() {
        let curve = FlatCurve { id: CurveId::new("USD-OIS") };
        let base = curve.base_date();
        let flows = vec![
            (base, Money::new(10.0, Currency::USD)),
            (base, Money::new(5.0, Currency::USD)),
        ];
        let pv = flows.npv(&curve, base, DayCount::Act365F).unwrap();
        assert!((pv.amount() - 15.0).abs() < 1e-12);
    }
}


