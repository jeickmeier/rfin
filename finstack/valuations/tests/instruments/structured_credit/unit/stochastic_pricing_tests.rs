//! Tests covering structured credit instrument-level stochastic helpers and loss math.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::PricingMode;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    DealType, Pool, PoolAsset, StructuredCredit, Tranche, TrancheCoupon, TrancheStructure,
};
use time::Month;

fn closing_date() -> Date {
    Date::from_calendar_date(2024, Month::January, 1).unwrap()
}

fn legal_maturity() -> Date {
    Date::from_calendar_date(2030, Month::January, 1).unwrap()
}

fn simple_pool(balance: f64) -> Pool {
    let mut pool = Pool::new("POOL", DealType::ABS, Currency::USD);
    if balance > 0.0 {
        pool.assets.push(PoolAsset::fixed_rate_bond(
            "A1",
            Money::new(balance, Currency::USD),
            0.06,
            Date::from_calendar_date(2029, Month::January, 1).unwrap(),
            finstack_core::dates::DayCount::Thirty360,
        ));
    }
    pool
}

fn single_tranche_structure(balance: f64) -> TrancheStructure {
    let tranche = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        finstack_valuations::instruments::fixed_income::structured_credit::Seniority::Senior,
        Money::new(balance, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        legal_maturity(),
    )
    .unwrap();
    TrancheStructure::new(vec![tranche]).unwrap()
}

fn discount_curve(base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (5.0, 0.95)])
        .build()
        .expect("discount curve")
}

fn build_sc(id: &str, pool_balance: f64) -> StructuredCredit {
    let pool = simple_pool(pool_balance);
    let tranches = single_tranche_structure(pool_balance);
    StructuredCredit::new_abs(
        id,
        pool,
        tranches,
        closing_date(),
        legal_maturity(),
        "USD-OIS",
    )
}

#[test]
fn stochastic_pricing_zero_notional_returns_zero_result() {
    let sc = build_sc("ABS-ZERO", 0.0);
    let mut market = MarketContext::new();
    market = market.insert_discount(discount_curve(closing_date()));

    let result = sc
        .price_stochastic_with_mode(&market, closing_date(), PricingMode::Tree)
        .expect("stochastic pricing");

    assert_eq!(result.npv.amount(), 0.0);
    assert_eq!(result.expected_loss.amount(), 0.0);
    assert!(
        result.tranche_results.is_empty(),
        "zero notional should skip tranche pricing"
    );
    assert_eq!(result.num_paths, 0);
}

#[test]
fn stochastic_pricing_is_deterministic_and_returns_tranche_results() {
    let sc = build_sc("ABS-DETERMINISTIC", 1_000_000.0);
    let mut market = MarketContext::new();
    market = market.insert_discount(discount_curve(closing_date()));

    let as_of = closing_date();
    let first = sc
        .price_stochastic_with_mode(&market, as_of, PricingMode::Tree)
        .expect("stochastic pricing");
    let second = sc
        .price_stochastic_with_mode(&market, as_of, PricingMode::Tree)
        .expect("stochastic pricing");

    assert!(first.npv.amount().is_finite());
    assert_eq!(first.tranche_results.len(), 1);
    assert_eq!(first.pricing_mode, "Tree");
    assert_eq!(first.npv.amount(), second.npv.amount());
    assert_eq!(first.tranche_results.len(), second.tranche_results.len());
}

#[test]
fn current_loss_percentage_respects_defaults_and_recoveries() {
    let mut sc = build_sc("ABS-LOSS", 1_000_000.0);
    sc.pool.cumulative_defaults = Money::new(100_000.0, Currency::USD);
    sc.pool.cumulative_recoveries = Money::new(25_000.0, Currency::USD);

    let loss_pct = sc.current_loss_percentage().expect("loss percentage");
    // Original balance ≈ current(1M) + defaults(100k) + prepays(0) = 1.1M
    // Net loss = 100k - 25k = 75k => 75k / 1.1M * 100 ≈ 6.818%
    let expected = (100_000.0 - 25_000.0) / 1_100_000.0 * 100.0;
    assert!(
        (loss_pct - expected).abs() < 1e-9,
        "expected {expected}%, got {loss_pct}"
    );
}
