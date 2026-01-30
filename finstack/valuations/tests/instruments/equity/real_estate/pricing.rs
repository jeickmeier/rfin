//! Pricing tests for real estate assets.

use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::real_estate::{
    RealEstateAsset, RealEstateValuationMethod,
};
use finstack_valuations::instruments::{Attributes, InstrumentNpvExt};
use finstack_valuations::test_utils::date;

#[test]
fn test_real_estate_dcf_pricing() {
    let valuation_date = date(2025, 1, 1);
    let noi1 = date(2026, 1, 1);
    let noi2 = date(2027, 1, 1);

    let asset = RealEstateAsset::builder()
        .id(InstrumentId::new("RE-DCF"))
        .currency(Currency::USD)
        .valuation_date(valuation_date)
        .valuation_method(RealEstateValuationMethod::Dcf)
        .noi_schedule(vec![(noi1, 100.0), (noi2, 100.0)])
        .discount_rate_opt(Some(0.10))
        .terminal_cap_rate_opt(Some(0.08))
        .day_count(DayCount::Act365F)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let ctx = MarketContext::new();
    let pv = asset.npv(&ctx, valuation_date).expect("npv");

    let t1 = DayCount::Act365F
        .year_fraction(valuation_date, noi1, DayCountCtx::default())
        .unwrap();
    let t2 = DayCount::Act365F
        .year_fraction(valuation_date, noi2, DayCountCtx::default())
        .unwrap();
    let pv_flows = 100.0 / (1.0_f64 + 0.10).powf(t1) + 100.0 / (1.0_f64 + 0.10).powf(t2);
    let terminal_value = 100.0 / 0.08;
    let pv_terminal = terminal_value / (1.0_f64 + 0.10).powf(t2);
    let expected = pv_flows + pv_terminal;

    // Allow small tolerance for floating point differences
    assert!(
        (pv.amount() - expected).abs() < 0.01,
        "PV={} vs expected={}",
        pv.amount(),
        expected
    );
}

#[test]
fn test_real_estate_direct_cap_pricing() {
    let valuation_date = date(2025, 1, 1);
    let noi1 = date(2026, 1, 1);

    let asset = RealEstateAsset::builder()
        .id(InstrumentId::new("RE-CAP"))
        .currency(Currency::USD)
        .valuation_date(valuation_date)
        .valuation_method(RealEstateValuationMethod::DirectCap)
        .noi_schedule(vec![(noi1, 120.0)])
        .cap_rate_opt(Some(0.06))
        .day_count(DayCount::Act365F)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let ctx = MarketContext::new();
    let pv = asset.npv(&ctx, valuation_date).expect("npv");

    let expected = 120.0 / 0.06;
    assert!((pv.amount() - expected).abs() < 1e-10);
}

#[test]
fn test_real_estate_appraisal_override() {
    let valuation_date = date(2025, 1, 1);
    let noi1 = date(2026, 1, 1);

    let asset = RealEstateAsset::builder()
        .id(InstrumentId::new("RE-APPRAISAL"))
        .currency(Currency::USD)
        .valuation_date(valuation_date)
        .valuation_method(RealEstateValuationMethod::Dcf)
        .noi_schedule(vec![(noi1, 100.0)])
        .discount_rate_opt(Some(0.10))
        .appraisal_value_opt(Some(Money::new(1_500.0, Currency::USD)))
        .day_count(DayCount::Act365F)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()
        .expect("should build");

    let ctx = MarketContext::new();
    let pv = asset.npv(&ctx, valuation_date).expect("npv");

    assert_eq!(pv.amount(), 1_500.0);
}
