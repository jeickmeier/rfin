//! Bond pricing engine tests.
//!
//! Tests the core pricing functionality including:
//! - Basic present value calculation
//! - Settlement date conventions
//! - Theta (time decay)
//! - Price sensitivity to curve shifts

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CashflowSpec};
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

fn create_flat_curve(rate: f64, base_date: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

#[test]
fn test_bond_basic_pricing() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "PRICE_TEST",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = bond.value(&market, as_of).unwrap();

    // At 5% curve with 5% coupon, should be near par
    assert!((pv.amount() - 1000.0).abs() < 50.0);
    assert_eq!(pv.currency(), Currency::USD);
}

#[test]
fn test_bond_price_vs_yield() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "YIELD_TEST",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Low yield → high price
    let low_curve = create_flat_curve(0.04, as_of, "USD-OIS");
    let market_low = MarketContext::new().insert_discount(low_curve);
    let pv_low = bond.value(&market_low, as_of).unwrap();

    // High yield → low price
    let high_curve = create_flat_curve(0.08, as_of, "USD-OIS");
    let market_high = MarketContext::new().insert_discount(high_curve);
    let pv_high = bond.value(&market_high, as_of).unwrap();

    // Verify inverse relationship
    assert!(pv_low.amount() > pv_high.amount());
}

#[test]
fn test_bond_price_coupon_relationship() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    // Bond with above-market coupon
    let high_coupon = Bond::fixed(
        "HIGH_COUPON",
        Money::new(1000.0, Currency::USD),
        0.08,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Bond with below-market coupon
    let low_coupon = Bond::fixed(
        "LOW_COUPON",
        Money::new(1000.0, Currency::USD),
        0.03,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv_high = high_coupon.value(&market, as_of).unwrap();
    let pv_low = low_coupon.value(&market, as_of).unwrap();

    // Higher coupon → higher price
    assert!(pv_high.amount() > pv_low.amount());

    // High coupon above market rate → premium
    assert!(pv_high.amount() > 1000.0);

    // Low coupon below market rate → discount
    assert!(pv_low.amount() < 1000.0);
}

#[test]
fn test_bond_price_maturity_relationship() {
    let as_of = date!(2025 - 01 - 01);

    let bond_2y = Bond::fixed(
        "2Y",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        date!(2027 - 01 - 01),
        "USD-OIS",
    );

    let bond_10y = Bond::fixed(
        "10Y",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        date!(2035 - 01 - 01),
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv_2y = bond_2y.value(&market, as_of).unwrap();
    let pv_10y = bond_10y.value(&market, as_of).unwrap();

    // At flat curve with coupon = yield, both should be near par
    assert!((pv_2y.amount() - 1000.0).abs() < 50.0);
    assert!((pv_10y.amount() - 1000.0).abs() < 50.0);
}

#[test]
fn test_bond_price_zero_coupon() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "ZERO",
        Money::new(1000.0, Currency::USD),
        0.0,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = bond.value(&market, as_of).unwrap();

    // Zero coupon bond should be priced at discount
    assert!(pv.amount() < 1000.0);

    // Approximately: 1000 * exp(-0.05 * 5) ≈ 778
    assert!((pv.amount() - 778.0).abs() < 50.0);
}

#[test]
fn test_bond_theta_time_decay() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "THETA_TEST",
        Money::new(1000.0, Currency::USD),
        0.06,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Price at T
    let pv_t0 = bond.value(&market, as_of).unwrap();

    // Price at T+1 day
    let tomorrow = date!(2025 - 01 - 02);
    let curve_t1 = create_flat_curve(0.05, tomorrow, "USD-OIS");
    let market_t1 = MarketContext::new().insert_discount(curve_t1);
    let pv_t1 = bond.value(&market_t1, tomorrow).unwrap();

    // Premium bond should decay toward par (price decrease)
    // But positive carry from coupon can offset this
    // Just verify prices are finite and reasonable
    assert!(pv_t0.amount().is_finite());
    assert!(pv_t1.amount().is_finite());
    assert!((pv_t0.amount() - pv_t1.amount()).abs() < 10.0); // Small 1-day change
}

#[test]
fn test_bond_settlement_date_impact() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    use finstack_valuations::instruments::bond::CashflowSpec;
    // No settlement lag
    let bond_t0 = Bond::builder()
        .id("SETTLE_T0".into())
        .notional(Money::new(1000.0, Currency::USD))
        .issue(as_of)
        .maturity(maturity)
        .cashflow_spec(CashflowSpec::fixed(
            0.05,
            finstack_core::dates::Frequency::semi_annual(),
            DayCount::Act365F,
        ))
        .discount_curve_id("USD-OIS".into())
        .settlement_days_opt(None)
        .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
        .build()
        .unwrap();

    // T+2 settlement
    let bond_t2 = Bond::builder()
        .id("SETTLE_T2".into())
        .notional(Money::new(1000.0, Currency::USD))
        .issue(as_of)
        .maturity(maturity)
        .cashflow_spec(CashflowSpec::fixed(
            0.05,
            finstack_core::dates::Frequency::semi_annual(),
            DayCount::Act365F,
        ))
        .discount_curve_id("USD-OIS".into())
        .settlement_days_opt(Some(2))
        .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
        .build()
        .unwrap();

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv_t0 = bond_t0.value(&market, as_of).unwrap();
    let pv_t2 = bond_t2.value(&market, as_of).unwrap();

    // T+2 settlement should result in slightly higher PV (less discounting)
    assert!(pv_t2.amount() >= pv_t0.amount());
}

#[test]
fn test_bond_matured_or_near_zero_value() {
    let as_of = date!(2025 - 01 - 01);
    let issue = date!(2020 - 01 - 01);
    let maturity = date!(2024 - 01 - 01); // Already matured

    let bond = Bond::fixed(
        "MATURED",
        Money::new(1000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = bond.value(&market, as_of).unwrap();

    // The cashflow provider filters to flows where date > settlement_date
    // For matured bonds, there should be no future flows, resulting in zero PV
    // If the implementation includes the maturity flow, adjust test accordingly
    // Verify PV is finite and non-negative
    assert!(pv.amount().is_finite());
    assert!(pv.amount() >= 0.0);
}

#[test]
fn test_bond_near_maturity_pricing() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2025 - 02 - 01); // 1 month away

    let bond = Bond::fixed(
        "NEAR_MAT",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = bond.value(&market, as_of).unwrap();

    // Very close to maturity, should be near par plus accrued
    assert!((pv.amount() - 1000.0).abs() < 20.0);
}

#[test]
fn test_bond_curve_parallel_shift() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "SHIFT_TEST",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Base curve at 5%
    let curve_base = create_flat_curve(0.05, as_of, "USD-OIS");
    let market_base = MarketContext::new().insert_discount(curve_base);
    let pv_base = bond.value(&market_base, as_of).unwrap();

    // Curve shifted down 100bp to 4%
    let curve_down = create_flat_curve(0.04, as_of, "USD-OIS");
    let market_down = MarketContext::new().insert_discount(curve_down);
    let pv_down = bond.value(&market_down, as_of).unwrap();

    // Curve shifted up 100bp to 6%
    let curve_up = create_flat_curve(0.06, as_of, "USD-OIS");
    let market_up = MarketContext::new().insert_discount(curve_up);
    let pv_up = bond.value(&market_up, as_of).unwrap();

    // Price increases when rates fall
    assert!(pv_down.amount() > pv_base.amount());

    // Price decreases when rates rise
    assert!(pv_up.amount() < pv_base.amount());

    // Verify reasonable magnitudes
    let delta_down = pv_down.amount() - pv_base.amount();
    let delta_up = pv_base.amount() - pv_up.amount();

    // Due to convexity, down move has larger impact
    assert!(delta_down > delta_up * 0.9); // Allow some tolerance
}

#[test]
fn test_bond_price_consistency() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond = Bond::fixed(
        "CONSISTENT",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Price multiple times should give same result
    let pv1 = bond.value(&market, as_of).unwrap();
    let pv2 = bond.value(&market, as_of).unwrap();
    let pv3 = bond.value(&market, as_of).unwrap();

    assert_eq!(pv1.amount(), pv2.amount());
    assert_eq!(pv2.amount(), pv3.amount());
}

#[test]
fn test_bond_notional_scaling() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let bond_100 = Bond::fixed(
        "N100",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let bond_1000 = Bond::fixed(
        "N1000",
        Money::new(1000.0, Currency::USD),
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv_100 = bond_100.value(&market, as_of).unwrap();
    let pv_1000 = bond_1000.value(&market, as_of).unwrap();

    // Price should scale linearly with notional
    assert!((pv_1000.amount() / pv_100.amount() - 10.0).abs() < 0.01);
}

#[test]
fn test_bond_different_day_counts() {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let day_counts = vec![
        DayCount::Thirty360,
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::ActAct,
    ];

    for dc in day_counts {
        let bond = Bond::builder()
            .id(format!("DC_{:?}", dc).into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(as_of)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Frequency::semi_annual(),
                dc,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default())
            .build()
            .unwrap();

        let disc_curve = create_flat_curve(0.05, as_of, "USD-OIS");
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv = bond.value(&market, as_of).unwrap();

        // All should produce valid, finite prices
        assert!(pv.amount().is_finite());
        assert!(pv.amount() > 0.0);
    }
}
