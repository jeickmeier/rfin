use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation, InflationLag};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::inflation_linked_bond::{DeflationProtection, InflationLinkedBond, IndexationMethod};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn ctx_with_index() -> (MarketContext, InflationIndex) {
    let disc = DiscountCurve::builder("USD-REAL")
        .base_date(d(2025, 1, 2))
        .knots([(0.0, 1.0), (0.5, 0.99), (1.0, 0.98)])
        .build()
        .unwrap();

    let mut index = InflationIndex::new("CPI-U", 300.0, d(2024, 12, 1)).unwrap();
    index = index.with_interpolation(InflationInterpolation::Linear);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation_index(index.clone());

    (ctx, index)
}

fn ctx_with_curve() -> (MarketContext, InflationCurve) {
    let disc = DiscountCurve::builder("USD-REAL")
        .base_date(d(2025, 1, 2))
        .knots([(0.0, 1.0), (0.5, 0.99), (1.0, 0.98)])
        .build()
        .unwrap();

    let curve = InflationCurve::builder("US-CPI")
        .base_date(d(2024, 12, 1))
        .base_cpi(300.0)
        .knots([(0.0, 300.0), (0.5, 303.0), (1.0, 306.0)])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(curve.clone());

    (ctx, curve)
}

fn sample_ilb() -> InflationLinkedBond {
    InflationLinkedBond::builder()
        .id(InstrumentId::new("ILB-TEST"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .real_coupon(0.01)
        .freq(Frequency::semi_annual())
        .dc(DayCount::ActAct)
        .issue(d(2025, 1, 2))
        .maturity(d(2027, 1, 2))
        .base_index(300.0)
        .base_date(d(2024, 12, 1))
        .indexation_method(IndexationMethod::TIPS)
        .lag(IndexationMethod::TIPS.standard_lag())
        .deflation_protection(DeflationProtection::MaturityOnly)
        .bdc(finstack_core::dates::BusinessDayConvention::Following)
        .stub(finstack_core::dates::StubKind::None)
        .calendar_id(None)
        .disc_id(CurveId::new("USD-REAL"))
        .inflation_id(CurveId::new("US-CPI"))
        .quoted_clean(Some(100.0))
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap()
}

#[test]
fn index_ratio_matches_index_and_curve_sources() {
    let ilb = sample_ilb();
    let (ctx_idx, idx) = ctx_with_index();
    let (ctx_curve, curve) = ctx_with_curve();

    // Ensure interpolation policies satisfy method
    assert_eq!(idx.interpolation(), InflationInterpolation::Linear);

    // Using index
    let r_idx = ilb.index_ratio(d(2025, 4, 1), &idx).unwrap();
    // Using curve
    let r_curve = ilb.index_ratio_from_curve(d(2025, 4, 1), &curve).unwrap();

    // Both sources should be consistent for aligned base and growth
    assert!((r_idx - r_curve).abs() < 1e-6);

    // Market-context routed
    let r_mkt = ilb.index_ratio_from_market(d(2025, 4, 1), &ctx_idx).unwrap();
    assert!((r_mkt - r_idx).abs() < 1e-12);
}

#[test]
fn real_yield_real_duration_and_breakeven_metrics() {
    let ilb = sample_ilb();
    let (ctx, _idx) = ctx_with_index();
    let as_of = d(2025, 1, 2);

    // Base pricing
    let pv = ilb.value(&ctx, as_of).unwrap();
    assert_eq!(pv.currency(), Currency::USD);

    // Metrics
    let metrics = [
        MetricId::RealYield,
        MetricId::RealDuration,
        MetricId::BreakevenInflation,
        MetricId::IndexRatio,
        MetricId::Dv01,
    ];
    let res = ilb.price_with_metrics(&ctx, as_of, &metrics).unwrap();

    // Real yield finite
    assert!(res.measures[MetricId::RealYield.as_str()].is_finite());
    // Duration sign check: price decreases when yield increases
    let dur = res.measures[MetricId::RealDuration.as_str()];
    assert!(dur > 0.0);
    // Breakeven ≈ nominal - real (use a nominal 3%)
    let be = res.measures[MetricId::BreakevenInflation.as_str()];
    assert!((be - (0.03 - res.measures[MetricId::RealYield.as_str()])).abs() < 1e-2);
    // Index ratio positive
    assert!(res.measures[MetricId::IndexRatio.as_str()] > 0.0);
}

#[test]
fn deflation_floor_applies_at_maturity() {
    let mut ilb = sample_ilb();
    // Force a lower CPI at maturity to test floor
    ilb.base_index = 300.0;
    let (mut ctx, mut idx) = ctx_with_index();
    idx = idx.with_value(d(2024, 12, 1), 300.0).unwrap();
    idx = idx.with_value(d(2025, 1, 1), 299.0).unwrap();
    // Insert updated index
    ctx = ctx.insert_inflation_index(idx.clone());

    // For maturity date, ratio must be floored to 1.0 under MaturityOnly
    let ratio = ilb.index_ratio(ilb.maturity, &idx).unwrap();
    assert!(ratio >= 1.0);
}


