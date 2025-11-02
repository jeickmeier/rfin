//! Accrued interest calculator tests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn create_curve(base_date: Date) -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_accrued_at_issue() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "ACCR1",
        Money::new(100.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    let market = create_curve(as_of);
    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Accrued])
        .unwrap();
    let accrued = *result.measures.get("accrued").unwrap();
    assert_eq!(accrued, 0.0);
}

#[test]
fn test_accrued_mid_period() {
    let issue = date!(2025 - 01 - 01);
    let mid = date!(2025 - 04 - 01); // ~3 months later
    let bond = Bond::fixed(
        "ACCR2",
        Money::new(100.0, Currency::USD),
        0.06,
        issue,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );
    let market = create_curve(mid);
    let result = bond
        .price_with_metrics(&market, mid, &[MetricId::Accrued])
        .unwrap();
    let accrued = *result.measures.get("accrued").unwrap();
    assert!(accrued > 0.0 && accrued < 3.0); // Semi-annual 6% = 3% per period
}

#[test]
fn test_accrued_frn_uses_forward_rate() {
    // FRN accrued should use forward rate from the last reset period
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::ForwardCurve;

    let issue = date!(2025 - 01 - 01);
    let as_of = date!(2025 - 03 - 15); // mid-quarter

    // Build market with discount and forward curve
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (1.0, 0.99)])
        .build()
        .unwrap();
    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(issue)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.05), (10.0, 0.05)]) // 5% flat
        .build()
        .unwrap();
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    // Floating-rate bond with SOFR 3M (build with BondFloatSpec)
    use finstack_core::dates::{BusinessDayConvention, Frequency, StubKind};
    use finstack_core::types::CurveId;
    use finstack_valuations::instruments::bond::BondFloatSpec;
    use finstack_valuations::instruments::common::traits::Attributes;
    use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    let bond = Bond::builder()
        .id("FRN1".into())
        .notional(Money::new(100.0, Currency::USD))
        .coupon(0.0)
        .issue(issue)
        .maturity(date!(2026 - 01 - 01))
        .freq(Frequency::quarterly())
        .dc(DayCount::Act360)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .credit_curve_id_opt(None)
        .pricing_overrides(PricingOverrides::default())
        .float_opt(Some(BondFloatSpec {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            margin_bp: 0.0,
            gearing: 1.0,
            reset_lag_days: 2,
        }))
        .attributes(Attributes::new())
        .build()
        .unwrap();

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Accrued])
        .unwrap();
    let accrued = *result.measures.get("accrued").unwrap();

    // Expect non-zero accrued consistent with ~5% * fraction of quarter * notional
    assert!(accrued > 0.0, "FRN accrued should be > 0");
}

#[test]
fn test_clean_dirty_ex_coupon_parity() {
    // Clean = Dirty - Accrued; around ex-coupon date accrued should reset
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;

    let issue = date!(2025 - 01 - 01);
    // let coup = date!(2025 - 07 - 01);
    let as_of_before = date!(2025 - 06 - 28);
    let as_of_after = date!(2025 - 07 - 02);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()
        .unwrap();
    let market = MarketContext::new().insert_discount(curve);

    let bond = Bond::fixed(
        "B",
        Money::new(100.0, Currency::USD),
        0.06,
        issue,
        date!(2026 - 01 - 01),
        "USD-OIS",
    );

    // Before coupon date: accrued > 0
    let res_before = bond
        .price_with_metrics(
            &market,
            as_of_before,
            &[
                MetricId::Accrued,
                MetricId::DirtyPrice,
                MetricId::CleanPrice,
            ],
        )
        .unwrap();
    let acc_before = *res_before.measures.get("accrued").unwrap();
    // Use base value as dirty price when no quoted clean is provided
    let dirty_before = res_before.value.amount();
    let clean_before = *res_before.measures.get("clean_price").unwrap();
    assert!((clean_before - (dirty_before - acc_before)).abs() < 1e-2);

    // After coupon date: check parity and ensure accrued decreased
    let res_after = bond
        .price_with_metrics(
            &market,
            as_of_after,
            &[MetricId::Accrued, MetricId::CleanPrice],
        )
        .unwrap();
    let acc_after = *res_after.measures.get("accrued").unwrap();
    let clean_after = *res_after.measures.get("clean_price").unwrap();
    let dirty_after = res_after.value.amount();
    assert!((clean_after - (dirty_after - acc_after)).abs() < 1e-2);
    assert!(
        acc_after < acc_before,
        "Accrued should decrease after coupon"
    );
}
