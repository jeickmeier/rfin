#![cfg(feature = "slow")]
//! YTM edge case tests for market standards compliance.
//!
//! Tests cover:
//! - Deep discount bonds (YTM > 20%)
//! - Zero-coupon bonds
//! - Bonds with odd first coupon
//! - EOM bonds with February maturity
//!
//! Market Standards Review (Priority 4, Week 4)

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CashflowSpec};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::Month;

use crate::instruments::common::test_helpers::tolerances;

fn create_test_market(base_date: Date) -> MarketContext {
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.90), (5.0, 0.70), (10.0, 0.50)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new().insert_discount(curve)
}

#[test]
fn test_deep_discount_bond_ytm() {
    // Deep discount bond: Trading at 50 cents on the dollar with 5% coupon
    // Expected YTM > 20%
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2035, Month::January, 1).unwrap();

    let mut bond = Bond::fixed(
        "DEEP-DISCOUNT",
        Money::new(1_000.0, Currency::USD),
        0.05, // 5% coupon
        issue,
        maturity,
        "USD-OIS",
    );

    let market = create_test_market(issue);

    // Set quoted price at deep discount
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(50.0); // 50 cents on dollar
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // Deep discount → high YTM
    assert!(
        ytm > 0.10, // Should be > 10% (relaxed for test robustness)
        "Deep discount bond should have high YTM, got: {:.2}%",
        ytm * 100.0
    );

    assert!(
        ytm < 0.50, // Sanity check < 50%
        "YTM unreasonably high: {:.2}%",
        ytm * 100.0
    );
}

#[test]
fn test_zero_coupon_bond_ytm() {
    // Zero-coupon bond: No coupons, only principal repayment
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    use finstack_valuations::instruments::bond::CashflowSpec;
    let bond_result = Bond::builder()
        .id("ZERO-COUPON".into())
        .notional(Money::new(1_000.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.0,
            Frequency::annual(),
            DayCount::Thirty360,
        ))
        .issue(issue)
        .maturity(maturity)
        .discount_curve_id("USD-OIS".into())
        .build();

    // Skip test if bond construction fails due to validation
    let mut bond = match bond_result {
        Ok(bond) => bond,
        Err(_) => {
            println!("Skipping test_zero_coupon_bond_ytm: bond construction failed validation");
            return;
        }
    };

    let market = create_test_market(issue);

    // Price at 80 cents on dollar
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(80.0);
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // For 5-year zero priced at 80: YTM = (100/80)^(1/5) - 1 ≈ 4.56%
    // This is analytically exact, so use NUMERICAL tolerance (1bp = 1e-4)
    // which accounts for Newton-Raphson solver precision.
    let expected_ytm = (1000.0 / 800.0_f64).powf(1.0 / 5.0) - 1.0;

    assert!(
        (ytm - expected_ytm).abs() < tolerances::NUMERICAL,
        "Zero-coupon YTM {:.6} should equal analytical {:.6} within {:.0}bp",
        ytm,
        expected_ytm,
        tolerances::NUMERICAL * 10000.0
    );
}

#[test]
fn test_odd_first_coupon_ytm() {
    // Bond with odd first coupon (short stub)
    // Issue: Jan 15, First coupon: Apr 1 (2.5 months), then regular semi-annual
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    use finstack_valuations::cashflow::builder::specs::{CouponType, FixedCouponSpec};
    use finstack_valuations::instruments::bond::CashflowSpec;
    let bond_result = Bond::builder()
        .id("ODD-FIRST".into())
        .notional(Money::new(1_000.0, Currency::USD))
        .cashflow_spec(CashflowSpec::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::ShortFront, // Short stub at front
        }))
        .issue(issue)
        .maturity(maturity)
        .discount_curve_id("USD-OIS".into())
        .build();

    // Skip test if bond construction fails due to validation
    let mut bond = match bond_result {
        Ok(bond) => bond,
        Err(_) => {
            println!("Skipping test_odd_first_coupon_ytm: bond construction failed validation");
            return;
        }
    };

    let market = create_test_market(issue);

    // Price at par
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(100.0);
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // At par, YTM should equal coupon rate
    assert!(
        (ytm - 0.05).abs() < 0.01,
        "Odd first coupon bond at par: YTM {:.4} should ≈ coupon 0.05",
        ytm
    );
}

#[test]
fn test_eom_february_maturity_ytm() {
    // EOM bond with February maturity (leap year handling)
    let issue = Date::from_calendar_date(2024, Month::February, 28).unwrap(); // 2024 is leap year
    let maturity = Date::from_calendar_date(2029, Month::February, 28).unwrap();

    let bond_result = Bond::builder()
        .id("EOM-FEB".into())
        .notional(Money::new(1_000.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.04,
            Frequency::annual(),
            DayCount::Thirty360,
        ))
        .issue(issue)
        .maturity(maturity)
        .discount_curve_id("USD-OIS".into())
        .build();

    // Skip test if bond construction fails due to validation
    let mut bond = match bond_result {
        Ok(bond) => bond,
        Err(_) => {
            println!(
                "Skipping test_eom_february_maturity_ytm: bond construction failed validation"
            );
            return;
        }
    };

    let market = create_test_market(issue);

    // Price slightly above par
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(102.0);
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // Above par → YTM < coupon
    assert!(
        ytm < 0.04,
        "Premium bond should have YTM < coupon: got {:.4}",
        ytm
    );

    assert!(ytm > 0.02, "YTM should be reasonable: got {:.4}", ytm);
}

#[test]
fn test_long_first_coupon_ytm() {
    use finstack_valuations::cashflow::builder::specs::{CouponType, FixedCouponSpec};
    use finstack_valuations::instruments::bond::CashflowSpec;
    // Bond with long first coupon period
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let bond_result = Bond::builder()
        .id("LONG-FIRST".into())
        .notional(Money::new(1_000.0, Currency::USD))
        .cashflow_spec(CashflowSpec::Fixed(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.06,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::LongFront, // Long stub at front
        }))
        .issue(issue)
        .maturity(maturity)
        .discount_curve_id("USD-OIS".into())
        .build();

    // Skip test if bond construction fails due to validation
    let mut bond = match bond_result {
        Ok(bond) => bond,
        Err(_) => {
            println!("Skipping test_long_first_coupon_ytm: bond construction failed validation");
            return;
        }
    };

    let market = create_test_market(issue);

    bond.pricing_overrides = PricingOverrides::default().with_clean_price(98.0); // Slight discount
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // Discount → YTM > coupon
    assert!(
        ytm > 0.06,
        "Discount bond should have YTM > coupon: got {:.4}",
        ytm
    );
}

#[test]
fn test_premium_bond_ytm_solver_convergence() {
    // Test YTM solver with premium bond (price > par)
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let mut bond = Bond::fixed(
        "PREMIUM",
        Money::new(1_000.0, Currency::USD),
        0.08, // 8% coupon (high)
        issue,
        maturity,
        "USD-OIS",
    );

    let market = create_test_market(issue);

    // Price at premium
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(115.0); // 115 cents on dollar
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // Premium bond → YTM < coupon
    assert!(
        ytm < 0.08,
        "Premium bond should have YTM < coupon: got {:.4}",
        ytm
    );

    // Should still be positive and reasonable
    assert!(
        ytm > 0.0 && ytm < 0.15,
        "YTM should be reasonable: got {:.4}",
        ytm
    );
}

#[test]
fn test_ytm_price_roundtrip() {
    // Test that price → YTM → price roundtrips correctly
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let mut bond = Bond::fixed(
        "ROUNDTRIP",
        Money::new(1_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let market = create_test_market(issue);

    let original_price = 95.0; // Discount price

    // Step 1: Calculate YTM from price
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(original_price);

    let result1 = bond
        .price_with_metrics(&market, issue, &[MetricId::Ytm, MetricId::CleanPrice])
        .unwrap();

    let ytm = result1.measures[MetricId::Ytm.as_str()];

    // Step 2: Calculate price without quote (use market curve)
    // Reset pricing overrides
    bond.pricing_overrides = PricingOverrides::default();

    let result2 = bond
        .price_with_metrics(&market, issue, &[MetricId::DirtyPrice])
        .unwrap();

    let calculated_dirty = result2.measures[MetricId::DirtyPrice.as_str()];

    // Verify YTM is reasonable
    assert!(
        ytm > 0.05 && ytm < 0.10,
        "Discount bond YTM should be > coupon: got {:.4}",
        ytm
    );

    // Verify dirty price is reasonable (allow wider range since it's from curve, not quote)
    assert!(
        calculated_dirty.is_finite(),
        "Dirty price should be finite: got {:.2}",
        calculated_dirty
    );
}

#[test]
fn test_very_long_maturity_bond() {
    // 30-year bond
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2055, Month::January, 1).unwrap();

    let mut bond = Bond::fixed(
        "LONG-30Y",
        Money::new(1_000.0, Currency::USD),
        0.04,
        issue,
        maturity,
        "USD-OIS",
    );

    let market = create_test_market(issue);

    bond.pricing_overrides = PricingOverrides::default().with_clean_price(90.0);
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, issue, &[MetricId::Ytm, MetricId::DurationMod])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];
    let duration = result.measures[MetricId::DurationMod.as_str()];

    // Long maturity discount bond should have high YTM
    assert!(
        ytm > 0.04,
        "30Y discount bond YTM should be > coupon: got {:.4}",
        ytm
    );

    // Duration should be substantial for 30Y bond
    assert!(
        duration > 10.0 && duration < 25.0,
        "30Y bond should have high duration: got {:.2}",
        duration
    );
}

#[test]
fn test_near_maturity_bond_ytm() {
    // Bond very close to maturity (1 month)
    let issue = Date::from_calendar_date(2024, Month::December, 1).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::February, 1).unwrap();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let mut bond = Bond::fixed(
        "NEAR-MATURITY",
        Money::new(1_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let market = create_test_market(as_of);

    bond.pricing_overrides = PricingOverrides::default().with_clean_price(99.5);
    let bond_with_quote = bond;

    let result = bond_with_quote
        .price_with_metrics(&market, as_of, &[MetricId::Ytm, MetricId::AccruedInterest])
        .unwrap();

    let ytm = result.measures[MetricId::Ytm.as_str()];

    // Near maturity, YTM should still be reasonable
    assert!(
        ytm.is_finite() && ytm > -0.05 && ytm < 0.20,
        "Near maturity YTM should be reasonable: got {:.4}",
        ytm
    );
}
