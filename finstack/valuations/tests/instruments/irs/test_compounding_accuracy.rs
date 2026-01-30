use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::{BusinessDayConvention, Date, DateExt, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::Currency;
use finstack_valuations::instruments::rates::irs::{
    FixedLegSpec, FloatLegSpec, FloatingLegCompounding, PayReceive,
};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::InterestRateSwap;
use time::Month;

#[test]
fn test_compounding_lookback_sensitivity() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let cal = CalendarRegistry::global()
        .resolve_str("USNY")
        .expect("USNY calendar");
    // Use a spot-starting swap (T+2) so SOFR lookback does not require historical fixings
    // at valuation date `base`.
    let start = base.add_business_days(2, cal).unwrap();

    // Flat 5% discount curve
    let disc = DiscountCurve::builder("DISC")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.951229)]) // exp(-0.05 * 1)
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    // Increasing forward curve: starts at 5%, ends at 10%
    let fwd = ForwardCurve::builder("FWD", 0.0)
        .base_date(base)
        .knots([(0.0, 0.05), (1.0, 0.10)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc.clone())
        .insert_forward(fwd.clone());

    // If lookback pushes observations before `as_of`, provide minimal fixings so the test
    // remains focused on the lookback sensitivity (not fixing-data availability).
    let earliest_obs = start
        .add_business_days(-2, cal)
        .unwrap_or(start.add_weekdays(-2));
    let ctx = if earliest_obs < base {
        let mut obs: Vec<(Date, f64)> = Vec::new();
        let mut d = earliest_obs;
        while d < base {
            obs.push((d, 0.05));
            d = d.add_business_days(1, cal).unwrap();
        }
        ctx.insert_series(ScalarTimeSeries::new("FIXING:FWD", obs, None).unwrap())
    } else {
        ctx
    };

    let mut irs = InterestRateSwap::builder()
        .id("TEST-COMP".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(FixedLegSpec {
            discount_curve_id: "DISC".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(12),
            payment_delay_days: 0,
            stub: finstack_core::dates::StubKind::None,
            par_method: None,
            compounding_simple: true,
        })
        .float(FloatLegSpec {
            discount_curve_id: "DISC".into(),
            forward_curve_id: "FWD".into(),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(12),
            compounding: FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 0,
                observation_shift: None,
            },
            payment_delay_days: 0,
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            fixing_calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            reset_lag_days: 0,
        })
        .build()
        .unwrap();

    // NPV with zero lookback
    let npv_no_lookback = irs.value(&ctx, base).unwrap().amount();

    // NPV with 2 days lookback
    // Since the curve is increasing, looking back 2 days should use LOWER rates,
    // so the floating leg PV should DECREASE, and NPV (Rec Fixed) should INCREASE.
    irs.float.compounding = FloatingLegCompounding::CompoundedInArrears {
        lookback_days: 2,
        observation_shift: None,
    };
    let npv_lookback = irs.value(&ctx, base).unwrap().amount();

    assert!(
        npv_lookback > npv_no_lookback,
        "Lookback in increasing rate env should increase RecFixed NPV (lower float PV)"
    );
}

#[test]
fn test_payment_delay_sensitivity() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("DISC")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("FWD", 0.0)
        .base_date(base)
        .knots([(0.0, 0.05), (1.0, 0.05)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Provide fixings for seasoned swap - example() creates a swap starting 2024-01-01
    // but we're pricing on 2025-01-01, so past resets need fixings
    let fixings = finstack_core::market_data::scalars::ScalarTimeSeries::new(
        "FIXING:FWD",
        vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                0.05,
            ),
            (
                Date::from_calendar_date(2024, Month::April, 1).unwrap(),
                0.05,
            ),
            (
                Date::from_calendar_date(2024, Month::July, 1).unwrap(),
                0.05,
            ),
            (
                Date::from_calendar_date(2024, Month::October, 1).unwrap(),
                0.05,
            ),
        ],
        None,
    )
    .expect("fixings series");

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_series(fixings);

    let mut irs = InterestRateSwap::example().unwrap();
    irs.fixed.discount_curve_id = "DISC".into();
    irs.float.discount_curve_id = "DISC".into();
    irs.float.forward_curve_id = "FWD".into();
    irs.float.compounding = FloatingLegCompounding::Simple; // Standard swap

    // NPV with 0 payment delay
    irs.fixed.payment_delay_days = 0;
    irs.float.payment_delay_days = 0;
    let npv0 = irs.value(&ctx, base).unwrap().amount();

    // NPV with 2 days payment delay
    // Delaying payments in a positive rate environment should lower the PV of both legs.
    irs.fixed.payment_delay_days = 2;
    irs.float.payment_delay_days = 2;
    let npv2 = irs.value(&ctx, base).unwrap().amount();

    // Since it's a par-like swap, the net effect depends on leg durations.
    // But the absolute leg PVs must change.
    assert!(npv2 != npv0, "Payment delay must change swap NPV");
}

#[test]
fn test_seasoned_compounded_swap_requires_fixings() {
    let start = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let cal = CalendarRegistry::global()
        .resolve_str("USNY")
        .expect("USNY calendar");
    let as_of = start.add_business_days(5, cal).unwrap();

    let disc = DiscountCurve::builder("DISC")
        .base_date(as_of)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("FWD", 0.0)
        .base_date(as_of)
        .knots([(0.0, 0.05), (1.0, 0.05)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    let irs = InterestRateSwap::builder()
        .id("TEST-SEASONED-COMP".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(FixedLegSpec {
            discount_curve_id: "DISC".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(1),
            payment_delay_days: 0,
            stub: finstack_core::dates::StubKind::None,
            par_method: None,
            compounding_simple: true,
        })
        .float(FloatLegSpec {
            discount_curve_id: "DISC".into(),
            forward_curve_id: "FWD".into(),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(1),
            compounding: FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 0,
                observation_shift: None,
            },
            payment_delay_days: 0,
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            fixing_calendar_id: Some("USNY".into()),
            stub: finstack_core::dates::StubKind::None,
            reset_lag_days: 0,
        })
        .build()
        .unwrap();

    let err = irs
        .value(&ctx, as_of)
        .expect_err("seasoned compounded swap must require explicit fixings");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("FIXING:FWD"),
        "error should explain expected fixings series id; got: {msg}"
    );
}

#[test]
fn test_seasoned_compounded_swap_with_fixings_prices() {
    let start = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let cal = CalendarRegistry::global()
        .resolve_str("USNY")
        .expect("USNY calendar");
    let as_of = start.add_business_days(5, cal).unwrap();

    let disc = DiscountCurve::builder("DISC")
        .base_date(as_of)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("FWD", 0.0)
        .base_date(as_of)
        .knots([(0.0, 0.05), (1.0, 0.05)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Build minimal business-day fixings from start up to (but not including) as_of.
    let mut obs: Vec<(Date, f64)> = Vec::new();
    let mut d = start;
    while d < as_of {
        obs.push((d, 0.05));
        d = d.add_business_days(1, cal).unwrap();
    }
    let fixings = ScalarTimeSeries::new("FIXING:FWD", obs, None).unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_series(fixings);

    let irs = InterestRateSwap::builder()
        .id("TEST-SEASONED-COMP-WITH-FIX".into())
        .notional(Money::new(10_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(FixedLegSpec {
            discount_curve_id: "DISC".into(),
            rate: rust_decimal::Decimal::try_from(0.05).expect("valid"),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(1),
            payment_delay_days: 0,
            stub: finstack_core::dates::StubKind::None,
            par_method: None,
            compounding_simple: true,
        })
        .float(FloatLegSpec {
            discount_curve_id: "DISC".into(),
            forward_curve_id: "FWD".into(),
            freq: Tenor::annual(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("USNY".into()),
            start,
            end: start.add_months(1),
            compounding: FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 0,
                observation_shift: None,
            },
            payment_delay_days: 0,
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            fixing_calendar_id: Some("USNY".into()),
            stub: finstack_core::dates::StubKind::None,
            reset_lag_days: 0,
        })
        .build()
        .unwrap();

    let pv = irs.value(&ctx, as_of).unwrap().amount();
    assert!(pv.is_finite(), "PV should be finite");
}
