use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::components::pool::{AssetPool, PoolAsset};
use finstack_valuations::instruments::structured_credit::components::enums::CreditRating;
use finstack_valuations::instruments::structured_credit::components::enums::DealType;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

#[test]
fn pool_was_wam_and_diversity() {
    let mut pool = AssetPool::new("POOL", DealType::CLO, Currency::USD);

    // Add a floating rate loan with explicit spread for WAS
    let a1 = PoolAsset::floating_rate_loan(
        "L1",
        Money::new(5_000_000.0, Currency::USD),
        "SOFR-3M",
        400.0,
        d(2030, 1, 1),
    )
    .with_rating(CreditRating::BB)
    .with_obligor("OB1");

    // Add a fixed rate bond (WAS falls back to rate)
    let a2 = PoolAsset::fixed_rate_bond(
        "B1",
        Money::new(3_000_000.0, Currency::USD),
        0.07,
        d(2029, 1, 1),
    )
    .with_rating(CreditRating::B)
    .with_obligor("OB2");

    // Add another loan same obligor to exercise diversity score
    let a3 = PoolAsset::floating_rate_loan(
        "L2",
        Money::new(2_000_000.0, Currency::USD),
        "SOFR-3M",
        500.0,
        d(2028, 1, 1),
    )
    .with_rating(CreditRating::B)
    .with_obligor("OB1");

    pool.assets.push(a1);
    pool.assets.push(a2);
    pool.assets.push(a3);

    // Weighted average spread should be between min and max spreads
    let was_bp = pool.weighted_avg_spread();
    assert!(was_bp > 0.0);

    // Weighted average maturity should be positive and within a plausible range
    let wam = pool.weighted_avg_maturity(d(2025, 1, 1));
    assert!(wam > 0.0 && wam < 10.0);

    // Diversity score > 1 and < number of assets since two share same obligor
    let div = pool.diversity_score();
    assert!(div > 1.0 && div < 3.0);
}

#[test]
fn pool_remaining_term_and_default_marking() {
    let mut asset = PoolAsset::fixed_rate_bond(
        "B2",
        Money::new(1_000_000.0, Currency::USD),
        0.06,
        d(2027, 1, 1),
    );
    let as_of = d(2025, 1, 1);
    let rem = asset.remaining_term(as_of, DayCount::Act365F).unwrap();
    assert!(rem > 0.0);

    asset.default_with_recovery(Money::new(200_000.0, Currency::USD), d(2025, 6, 1));
    assert!(asset.is_defaulted);
    assert!(asset.recovery_amount.is_some());
}


