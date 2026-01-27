//! Compatibility layer for discounting instrument cashflow schedules.

pub use finstack_core::cashflow::Discountable;

use crate::cashflow::builder::CashFlowSchedule;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;

impl Discountable for CashFlowSchedule {
    type PVOutput = finstack_core::Result<Money>;

    fn npv(
        &self,
        disc: &dyn Discounting,
        base: Date,
        dc: Option<DayCount>,
    ) -> finstack_core::Result<Money> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        finstack_core::cashflow::npv(disc, base, dc, &flows)
    }
}

/// Discount dated `Money` flows to `as_of` using the curve's own day-count and
/// date-based discount factor calculation (**holder-view** semantics).
///
/// # Cashflow-on-as_of Policy: HOLDER-VIEW (Excludes `d <= as_of`)
///
/// This helper treats valuation as occurring **just after settlement**:
/// - Cashflows where `d <= as_of` are considered already settled and are **excluded**
/// - Only future cashflows (`d > as_of`) contribute to NPV
///
/// ## When to Use
///
/// Use this for instruments where the holder has already received or paid any
/// cashflow due on `as_of`:
/// - **Term loans**: Interest accrued up to `as_of` is already paid
/// - **Bonds (dirty price)**: Coupon on `as_of` has been received
/// - **Seasoned swaps**: Past cashflows are settled
///
/// ## Alternative
///
/// For T+0 instruments or calibration where cashflows on `as_of` should be included,
/// use [`crate::instruments::common::helpers::schedule_pv_using_curve_dc_raw`] which
/// includes `d == as_of` cashflows (pricing-view semantics).
///
/// # Arguments
///
/// * `disc` - Discount curve for date-based DF lookup
/// * `as_of` - Valuation date (cashflows on or before this are excluded)
/// * `flows` - Vector of (date, amount) pairs
///
/// # Returns
///
/// Sum of discounted future cashflows (holder-view NPV).
pub fn npv_by_date(
    disc: &DiscountCurve,
    as_of: Date,
    flows: &[(Date, Money)],
) -> finstack_core::Result<Money> {
    if flows.is_empty() {
        return Err(finstack_core::InputError::TooFewPoints.into());
    }

    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);

    for (d, amt) in flows {
        // HOLDER-VIEW: exclude cashflows on or before as_of (already settled)
        if *d <= as_of {
            continue;
        }
        let df = disc.df_between_dates(as_of, *d)?;

        total = (total + (*amt * df))?;
    }

    Ok(total)
}

/// Discount dated `Money` flows to `as_of` (**pricing-view** semantics).
///
/// # Cashflow-on-as_of Policy: PRICING-VIEW (Includes `d == as_of`)
///
/// This helper includes cashflows occurring exactly on `as_of`:
/// - Cashflows where `d < as_of` are excluded (truly past)
/// - Cashflows where `d == as_of` are **included** at DF=1 (t=0)
/// - Future cashflows (`d > as_of`) are discounted
///
/// ## When to Use
///
/// Use this for instruments where cashflows on `as_of` are part of the pricing:
/// - **T+0 deposits**: Initial exchange occurs on valuation date
/// - **Calibration instruments**: Bracketing requires all cashflows
/// - **FRAs**: Settlement on as_of is part of the value
///
/// ## Alternative
///
/// For holder-view PV (excludes `d <= as_of`), use [`npv_by_date`].
///
/// # Arguments
///
/// * `disc` - Discount curve for date-based DF lookup
/// * `as_of` - Valuation date
/// * `flows` - Vector of (date, amount) pairs
///
/// # Returns
///
/// Sum of discounted cashflows (pricing-view NPV).
#[allow(dead_code)] // API completeness: kept alongside npv_by_date for pricing-view use cases
pub fn npv_by_date_pricing_view(
    disc: &DiscountCurve,
    as_of: Date,
    flows: &[(Date, Money)],
) -> finstack_core::Result<Money> {
    if flows.is_empty() {
        return Err(finstack_core::InputError::TooFewPoints.into());
    }

    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);

    for (d, amt) in flows {
        // PRICING-VIEW: exclude only truly past cashflows (d < as_of)
        // Include d == as_of (DF=1 at t=0)
        if *d < as_of {
            continue;
        }
        let df = disc.df_between_dates(as_of, *d)?;

        total = (total + (*amt * df))?;
    }

    Ok(total)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
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
            freq: Tenor::quarterly(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };
        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid rate"),
            freq: params.freq,
            dc: params.dc,
            bdc: params.bdc,
            calendar_id: params.calendar_id,
            stub: params.stub,
        };
        CashFlowSchedule::builder()
            .principal(Money::new(1_000.0, Currency::USD), issue, maturity)
            .fixed_cf(fixed)
            .build_with_curves(None)
            .expect("should build schedule")
    }

    #[test]
    fn schedule_discountable_paths_through() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let schedule = simple_schedule();
        // Use explicit day count
        let pv = schedule
            .npv(&curve, base, Some(DayCount::Act365F))
            .expect("should calculate NPV");
        assert!(pv.amount().is_finite());
    }

    // ==================== PV SEMANTICS TESTS ====================

    fn create_test_curve() -> finstack_core::market_data::term_structures::DiscountCurve {
        use finstack_core::market_data::term_structures::DiscountCurve;
        let base = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        DiscountCurve::builder("TEST")
            .base_date(base)
            .knots([(0.0, 1.0), (0.5, 0.98), (1.0, 0.95)])
            .build()
            .expect("should build")
    }

    #[test]
    fn holder_view_excludes_cashflow_on_as_of() {
        let disc = create_test_curve();
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let future = Date::from_calendar_date(2024, Month::July, 1).expect("valid date");

        // Flow exactly on as_of (should be EXCLUDED in holder-view)
        let flows = vec![
            (as_of, Money::new(100.0, Currency::USD)),  // on as_of
            (future, Money::new(100.0, Currency::USD)), // future
        ];

        let pv = npv_by_date(&disc, as_of, &flows).expect("should succeed");

        // Holder-view: only future flow should contribute
        // DF for 6 months ≈ 0.98
        assert!(
            pv.amount() > 90.0 && pv.amount() < 100.0,
            "Holder-view PV should only include future flow: {}",
            pv.amount()
        );

        // Specifically: should NOT be ~200 (both flows) or ~100 (just as_of flow)
        assert!(
            pv.amount() < 105.0,
            "Should exclude as_of cashflow: {}",
            pv.amount()
        );
    }

    #[test]
    fn pricing_view_includes_cashflow_on_as_of() {
        let disc = create_test_curve();
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let future = Date::from_calendar_date(2024, Month::July, 1).expect("valid date");

        // Flow exactly on as_of (should be INCLUDED in pricing-view)
        let flows = vec![
            (as_of, Money::new(100.0, Currency::USD)),  // on as_of
            (future, Money::new(100.0, Currency::USD)), // future
        ];

        let pv = npv_by_date_pricing_view(&disc, as_of, &flows).expect("should succeed");

        // Pricing-view: both flows should contribute
        // as_of flow: 100 * 1.0 = 100 (DF=1 at t=0)
        // future flow: 100 * ~0.98 = ~98
        // Total: ~198
        assert!(
            pv.amount() > 190.0 && pv.amount() < 200.0,
            "Pricing-view PV should include both flows: {}",
            pv.amount()
        );
    }

    #[test]
    fn holder_vs_pricing_view_difference() {
        let disc = create_test_curve();
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let future = Date::from_calendar_date(2024, Month::July, 1).expect("valid date");

        let flows = vec![
            (as_of, Money::new(100.0, Currency::USD)), // on as_of
            (future, Money::new(50.0, Currency::USD)), // future
        ];

        let pv_holder = npv_by_date(&disc, as_of, &flows).expect("holder-view");
        let pv_pricing = npv_by_date_pricing_view(&disc, as_of, &flows).expect("pricing-view");

        // Difference should be approximately the as_of cashflow (100)
        let diff = pv_pricing.amount() - pv_holder.amount();
        assert!(
            (diff - 100.0).abs() < 1.0,
            "Difference should be ~100 (the as_of cashflow): diff={}",
            diff
        );
    }

    #[test]
    fn both_views_exclude_past_cashflows() {
        let disc = create_test_curve();
        let as_of = Date::from_calendar_date(2024, Month::July, 1).expect("valid date");
        let past = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let future = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");

        let flows = vec![
            (past, Money::new(100.0, Currency::USD)),  // past (< as_of)
            (future, Money::new(50.0, Currency::USD)), // future
        ];

        let pv_holder = npv_by_date(&disc, as_of, &flows).expect("holder-view");
        let pv_pricing = npv_by_date_pricing_view(&disc, as_of, &flows).expect("pricing-view");

        // Both should only include the future flow (past is excluded in both)
        assert!(
            (pv_holder.amount() - pv_pricing.amount()).abs() < 0.01,
            "Both views should give same result when no as_of cashflow: holder={}, pricing={}",
            pv_holder.amount(),
            pv_pricing.amount()
        );
    }
}
