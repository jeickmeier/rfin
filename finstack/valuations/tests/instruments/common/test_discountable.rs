//! Tests for Discountable trait and NPV calculations.

use finstack_valuations::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams};
use finstack_valuations::instruments::common::Discountable;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::{Discounting, TermStructure};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use time::Month;

use crate::common::test_helpers::*;

struct FlatCurve {
    id: CurveId,
    rate: f64,
}

impl TermStructure for FlatCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Discounting for FlatCurve {
    fn base_date(&self) -> Date {
        test_date()
    }
    
    fn df(&self, t: f64) -> f64 {
        (-self.rate * t).exp() as f64
    }
}

#[test]
fn test_schedule_discountable_simple() {
    // Arrange
    let curve = FlatCurve {
        id: CurveId::new("USD-OIS"),
        rate: 0.05,
    };
    
    let issue = test_date();
    let maturity = Date::from_calendar_date(2025, Month::July, 1).unwrap();
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
    
    let schedule = CashFlowSchedule::builder()
        .principal(Money::new(1_000.0, Currency::USD), issue, maturity)
        .fixed_cf(fixed)
        .build()
        .unwrap();
    
    // Act
    let pv = schedule.npv(&curve, curve.base_date(), DayCount::Act365F).unwrap();
    
    // Assert
    assert!(pv.amount().is_finite(), "PV is finite");
    assert!(pv.amount() > 0.0, "PV is positive");
    assert_eq!(pv.currency(), Currency::USD);
}

#[test]
fn test_npv_zero_rate() {
    // Arrange: Zero rate means no discounting
    let curve = FlatCurve {
        id: CurveId::new("TEST"),
        rate: 0.0,
    };
    
    let issue = test_date();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let params = ScheduleParams {
        freq: Frequency::annual(),
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
    
    let schedule = CashFlowSchedule::builder()
        .principal(Money::new(1_000.0, Currency::USD), issue, maturity)
        .fixed_cf(fixed)
        .build()
        .unwrap();
    
    // Act
    let pv = schedule.npv(&curve, curve.base_date(), DayCount::Act365F).unwrap();
    
    // Assert: With zero rate, PV = sum of cashflows
    let expected = 1_000.0 + 1_000.0 * 0.05; // Principal + 1yr coupon
    assert_approx_eq(pv.amount(), expected, 1.0, "PV equals cashflow sum at 0%");
}

