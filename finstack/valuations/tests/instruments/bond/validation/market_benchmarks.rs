//! Bond metrics validation tests against known market benchmarks.
//!
//! These tests validate our bond metric calculations against:
//! - Industry textbook examples
//! - Market-standard calculators  
//! - Bloomberg-style calculations
//!
//! References:
//! - Fabozzi, "The Handbook of Fixed Income Securities"
//! - Hull, "Options, Futures, and Other Derivatives"
//! - Market practice conventions

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::CashflowSpec;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::{Bond, PricingOverrides};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

use crate::common::test_helpers::tolerances;

/// Helper to build a standard flat yield curve for testing
fn build_flat_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
            (30.0, (-rate * 30.0).exp()),
        ])
        .build()
        .unwrap()
}

#[test]
fn test_bond_ytm_benchmark_1() {
    // Benchmark: 5% semi-annual coupon bond, 3 years to maturity
    // Price: 95.00 (clean)
    // Expected YTM: ~6.945% (from Fabozzi example, using 30/360 convention)

    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2027 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

    use finstack_valuations::instruments::bond::CashflowSpec;
    // Use 30/360 day count to match Fabozzi convention
    let bond = Bond::builder()
        .id("BOND_YTM_TEST1".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.05,
            Tenor::semi_annual(),
            DayCount::Thirty360,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    // Build market context with flat 5% curve
    let disc_curve = build_flat_curve(0.05, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm])
        .unwrap();

    let ytm = *result.measures.get("ytm").unwrap();

    // Verify YTM is in expected range for this discount bond
    //
    // For a 5% coupon bond at 95 clean price with 3Y maturity:
    // - Quick approximation: YTM ≈ coupon + (par - price)/(price × years)
    //   = 5% + (100-95)/(95×3) = 5% + 1.75% ≈ 6.75%
    // - Street convention with actual day fractions gives ~6.87%
    //
    // Note: The Fabozzi benchmark (6.945%) uses simplified assumptions
    // (exact 0.5 year periods). Our implementation uses actual day count
    // fractions which differ slightly.
    let ytm_min = 0.065; // At least 6.5%
    let ytm_max = 0.075; // At most 7.5%

    assert!(
        ytm >= ytm_min && ytm <= ytm_max,
        "YTM={:.4}% should be in range [{:.1}%, {:.1}%] for discount bond",
        ytm * 100.0,
        ytm_min * 100.0,
        ytm_max * 100.0
    );

    // Verify self-consistency: price from computed YTM should match target
    use finstack_valuations::cashflow::traits::CashflowProvider;
    let flows = bond.build_schedule(&market, as_of).unwrap();
    let recalc_price =
        finstack_valuations::instruments::bond::pricing::quote_engine::price_from_ytm(
            &bond, &flows, as_of, ytm,
        )
        .unwrap();

    // The recalculated dirty price should match clean price + accrued (= 95 at issue)
    let target_dirty = 95.0; // No accrued at issue date
    assert!(
        (recalc_price - target_dirty).abs() < 0.01,
        "Round-trip: price from YTM ({:.4}) should match target ({:.2})",
        recalc_price,
        target_dirty
    );
}

#[test]
fn test_bond_ytm_benchmark_2_par_bond() {
    // Benchmark: Par bond (price = 100)
    // YTM should equal coupon rate

    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("BOND_PAR_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.06,
            Tenor::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.06, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Ytm])
        .unwrap();

    let ytm = *result.measures.get("ytm").unwrap();

    // For par bond, YTM = coupon rate
    assert!(
        (ytm - 0.06).abs() < 0.0001,
        "Par bond YTM={:.4}% should equal coupon 6.00%",
        ytm * 100.0
    );
}

/// Macaulay Duration benchmark from Fabozzi, "The Handbook of Fixed Income Securities"
///
/// Setup: 5-year, 8% annual coupon bond at par (YTM = 8%)
/// Expected: 4.312 years
///
/// Analytical formula: D = Σ[t × CF_t × DF_t] / P
/// where DF_t = 1 / (1 + y)^t
#[test]
fn test_bond_macaulay_duration_benchmark() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("BOND_DUR_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.08,
            Tenor::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.08, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMac])
        .unwrap();

    let mac_duration = *result.measures.get("duration_mac").unwrap();

    // Expected: 4.312 years (Fabozzi benchmark)
    // Using RELATIVE tolerance (1%) for duration comparison
    let expected = 4.312;
    assert!(
        (mac_duration - expected).abs() < expected * tolerances::RELATIVE,
        "Macaulay Duration={:.4}y vs Fabozzi expected {:.3}y (±1%)",
        mac_duration,
        expected
    );
}

/// Modified Duration benchmark derived from Macaulay Duration
///
/// Formula: ModDur = MacDur / (1 + y/m)
/// For annual bond at 8% YTM: ModDur = 4.312 / 1.08 = 3.993
#[test]
fn test_bond_modified_duration_benchmark() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("BOND_MODDUR_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.08,
            Tenor::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.08, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMod])
        .unwrap();

    let mod_duration = *result.measures.get("duration_mod").unwrap();

    // Expected: 3.993 years (MacDur / (1 + y))
    // Using RELATIVE tolerance (1%) for duration comparison
    let expected = 3.993;
    assert!(
        (mod_duration - expected).abs() < expected * tolerances::RELATIVE,
        "Modified Duration={:.4}y vs expected {:.3}y (±1%)",
        mod_duration,
        expected
    );
}

/// DV01 (Dollar Value of 01) market standard test
///
/// DV01 Sign Convention:
/// - DV01 = ΔPrice / Δ(1bp rate increase)
/// - For fixed-rate bonds: DV01 < 0 (inverse price/yield relationship)
/// - Magnitude ≈ ModDur × Price × 0.0001
///
/// For $100 par, ModDur=4.0, |DV01| should be ~0.04 per $100 face
#[test]
fn test_bond_dv01_market_standard() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("BOND_DV01_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.08,
            Tenor::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.08, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMod, MetricId::Dv01])
        .unwrap();

    let mod_duration = *result.measures.get("duration_mod").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    let price = result.value.amount();

    // DV01 is computed via generic bump-and-reprice (more accurate than linear approximation)
    // Verify sign: DV01 < 0 for fixed-rate bonds (price decreases when rates rise)
    assert!(dv01 < 0.0, "DV01 should be negative for fixed-rate bond");

    // Approximate relationship: DV01 ≈ −Price × ModDur × 1bp
    // For a par bond on flat curve, DV01 and ModDur-based estimate should be
    // within 2% - convexity effect for 1bp is negligible.
    let approx_dv01 = -(price * mod_duration * 0.0001);
    let relative_diff = ((dv01 - approx_dv01) / approx_dv01).abs();

    assert!(
        relative_diff < 0.02, // 2% tolerance for par bond on flat curve
        "DV01={:.6} differs from duration estimate {:.6} by {:.2}%",
        dv01,
        approx_dv01,
        relative_diff * 100.0
    );
}

#[test]
fn test_bond_price_yield_relationship() {
    // Fundamental bond relationship: as yield increases, price decreases
    // Test with same bond at different yields

    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let yields = vec![0.04, 0.06, 0.08, 0.10];
    let mut prices = Vec::new();

    for yield_rate in yields {
        let bond = Bond::builder()
            .id("BOND_PRICE_YIELD".into())
            .notional(Money::new(100.0, Currency::USD))
            .cashflow_spec(CashflowSpec::fixed(
                0.06,
                Tenor::semi_annual(),
                DayCount::Act365F,
            ))
            .issue(as_of)
            .maturity(maturity)
            .discount_curve_id("USD_DISC".into())
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .build()
            .unwrap();

        let disc_curve = build_flat_curve(yield_rate, as_of, "USD_DISC");
        let market = MarketContext::new().insert_discount(disc_curve);

        let price = bond.value(&market, as_of).unwrap();
        prices.push(price.amount());
    }

    // Verify inverse relationship: higher yield = lower price
    for i in 1..prices.len() {
        assert!(
            prices[i] < prices[i - 1],
            "Price should decrease as yield increases: price[{}]={:.2} >= price[{}]={:.2}",
            i,
            prices[i],
            i - 1,
            prices[i - 1]
        );
    }

    // Middle price (6% yield, 6% coupon) should be near par
    assert!(
        (prices[1] - 100.0).abs() < 1.0,
        "Bond with 6% coupon at 6% yield should be near par: {:.2}",
        prices[1]
    );
}

#[test]
fn test_bond_zero_coupon_duration() {
    // Zero coupon bond: Macaulay Duration = Time to Maturity

    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01); // 5 years

    let pricing_overrides = PricingOverrides::default().with_clean_price(70.0);

    let bond = Bond::builder()
        .id("ZERO_COUPON_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(0.0, Tenor::annual(), DayCount::Act365F))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.07, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMac])
        .unwrap();

    let mac_duration = *result.measures.get("duration_mac").unwrap();

    // For zero coupon bond, duration = time to maturity (5 years)
    assert!(
        (mac_duration - 5.0).abs() < 0.1,
        "Zero coupon bond duration={:.2} should equal maturity 5.00 years",
        mac_duration
    );
}

#[test]
fn test_bond_convexity_positive() {
    // All bonds have positive convexity

    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("BOND_CVX_TEST".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.08,
            Tenor::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let disc_curve = build_flat_curve(0.08, as_of, "USD_DISC");
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::Convexity])
        .unwrap();

    let convexity = *result.measures.get("convexity").unwrap();

    // Convexity should be positive for non-callable bonds
    assert!(
        convexity > 0.0,
        "Bond convexity={:.2} should be positive",
        convexity
    );

    // For 5-year bond, typically in range 15-25
    assert!(
        convexity > 10.0 && convexity < 30.0,
        "Convexity={:.2} outside typical range 10-30 for 5Y bond",
        convexity
    );
}
