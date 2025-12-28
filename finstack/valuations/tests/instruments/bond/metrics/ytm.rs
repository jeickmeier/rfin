//! Yield to maturity calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

/// Par bond YTM should equal coupon rate
///
/// At par (price = 100), the yield to maturity equals the coupon rate.
/// Small deviations may occur due to compounding convention differences
/// between bond coupons (semi-annual) and curve (continuous).
#[test]
fn test_ytm_par_bond() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "YTM1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    ).unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm])
        .unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    // At par, YTM should equal coupon rate within 5bp
    // Small deviation allowed for compounding convention mismatch
    assert!(
        (ytm - 0.05).abs() < 0.0005,
        "Par bond YTM {:.6} should approximately equal coupon 0.05",
        ytm
    );
}

/// YTM should be well-defined and finite for a simple FRN, even though the
/// market-standard quote for FRNs is discount margin rather than YTM.
#[test]
fn test_ytm_floating_bond_is_finite_from_price() {
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Tenor};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::bond::Bond;
    use finstack_valuations::instruments::PricingOverrides;
    use finstack_valuations::metrics::MetricId;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Simple, smooth curves suitable for FRN pricing.
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (2.0, 0.95)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (2.0, 0.035)])
        .build()
        .unwrap();
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    let mut bond = Bond::floating(
        "YTM-FRN",
        notional,
        "USD-SOFR-3M",
        150.0,
        as_of,
        maturity,
        Tenor::quarterly(),
        DayCount::Act360,
        "USD-OIS",
    ).unwrap();

    // Use the model PV to infer a clean price quote (assuming valuation on a
    // coupon date so that accrued is approximately zero).
    let pv = bond.value(&market, as_of).unwrap().amount();
    let clean_px = pv / notional.amount() * 100.0;
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(clean_px);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm])
        .unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    assert!(
        ytm.is_finite(),
        "FRN YTM should be finite when solved from full cashflows and price"
    );
}

/// YTM should also be well-defined for simple amortizing structures, where it
/// is interpreted as the IRR of the full projected cashflow schedule.
#[test]
fn test_ytm_amortizing_bond_is_finite_from_price() {
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Tenor};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use finstack_valuations::instruments::bond::Bond;
    use finstack_valuations::instruments::bond::{AmortizationSpec, CashflowSpec};
    use finstack_valuations::instruments::common::traits::Attributes;
    use finstack_valuations::instruments::PricingOverrides;
    use finstack_valuations::metrics::MetricId;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Simple downward-sloping discount curve.
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (3.0, 0.94)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let market = MarketContext::new().insert_discount(disc);

    // Amortizing schedule: step down principal once before final maturity.
    let step_date = date!(2026 - 01 - 01);
    let amort_spec = AmortizationSpec::StepRemaining {
        schedule: vec![
            (step_date, Money::new(500_000.0, Currency::USD)),
            (maturity, Money::new(0.0, Currency::USD)),
        ],
    };
    let base_spec = CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360);
    let cashflow_spec = CashflowSpec::amortizing(base_spec, amort_spec);

    let mut bond = Bond::builder()
        .id("YTM-AMORT".into())
        .notional(notional)
        .issue(as_of)
        .maturity(maturity)
        .cashflow_spec(cashflow_spec)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .expect("amortizing bond construction should succeed in test");

    // Infer a clean price from the model PV at as_of.
    let pv = bond.value(&market, as_of).unwrap().amount();
    let clean_px = pv / notional.amount() * 100.0;
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(clean_px);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm])
        .unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    assert!(
        ytm.is_finite(),
        "Amortizing bond YTM should be finite when solved from full cashflows and price"
    );
}
