//! Compatibility layer for discounting instrument cashflow schedules.

pub use finstack_core::cashflow::discounting::{npv_static, Discountable};

use crate::cashflow::builder::CashFlowSchedule;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;

impl Discountable for CashFlowSchedule {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        finstack_core::cashflow::discounting::npv_static(disc, base, dc, &flows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
    use finstack_core::market_data::traits::{Discounting, TermStructure};
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;

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
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date")
        }
        fn df(&self, _t: f64) -> f64 {
            1.0
        }
    }

    fn simple_schedule() -> CashFlowSchedule {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let maturity = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
        let params = ScheduleParams {
            freq: Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };
        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: params.freq,
            dc: params.dc,
            bdc: params.bdc,
            calendar_id: params.calendar_id,
            stub: params.stub,
        };
        CashFlowSchedule::builder()
            .principal(Money::new(1_000.0, Currency::USD), issue, maturity)
            .fixed_cf(fixed)
            .build()
            .expect("should build schedule")
    }

    #[test]
    fn schedule_discountable_paths_through() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let schedule = simple_schedule();
        let pv = schedule.npv(&curve, base, DayCount::Act365F).expect("should calculate NPV");
        assert!(pv.amount().is_finite());
    }
}
