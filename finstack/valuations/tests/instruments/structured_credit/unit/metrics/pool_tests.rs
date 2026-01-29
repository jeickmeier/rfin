//! Unit tests for pool characteristic metrics.
//!
//! Tests cover:
//! - WAC/WAS calculations
//! - Default rate aggregation
//! - Basic pool stats output consistency

use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    calculate_pool_stats, DealType, Pool, PoolAsset,
};
use time::Month;

fn maturity_date() -> finstack_core::dates::Date {
    finstack_core::dates::Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

#[test]
fn test_pool_stats_weighted_spread_and_coupon() {
    let mut pool = Pool::new("POOL", DealType::CLO, Currency::USD);
    pool.assets.push(PoolAsset::floating_rate_loan(
        "L1",
        Money::new(10_000_000.0, Currency::USD),
        "SOFR-3M",
        400.0,
        maturity_date(),
        DayCount::Act360,
    ));
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "B1",
        Money::new(20_000_000.0, Currency::USD),
        0.06,
        maturity_date(),
        DayCount::Thirty360,
    ));

    let stats = calculate_pool_stats(&pool, maturity_date());

    let expected_was = (10_000_000.0 * 400.0 + 20_000_000.0 * 600.0) / 30_000_000.0;
    assert!(
        (stats.weighted_avg_spread - expected_was).abs() < 1e-6,
        "Weighted avg spread should match balance-weighted spreads"
    );
    assert!(
        stats.weighted_avg_coupon > 0.0,
        "Weighted avg coupon should be positive for fixed/floating assets"
    );
}

#[test]
fn test_pool_stats_default_rate() {
    let mut pool = Pool::new("POOL", DealType::CLO, Currency::USD);
    let mut defaulted = PoolAsset::fixed_rate_bond(
        "D1",
        Money::new(5_000_000.0, Currency::USD),
        0.05,
        maturity_date(),
        DayCount::Thirty360,
    );
    defaulted.is_defaulted = true;

    pool.assets.push(defaulted);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "D2",
        Money::new(10_000_000.0, Currency::USD),
        0.05,
        maturity_date(),
        DayCount::Thirty360,
    ));

    let stats = calculate_pool_stats(&pool, maturity_date());
    assert!(
        (stats.cumulative_default_rate - 33.333333).abs() < 1e-3,
        "Default rate should reflect defaulted balance share"
    );
}
