use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::components::pool::{calculate_pool_stats, AssetPool, PoolAsset};
use finstack_valuations::instruments::structured_credit::components::waterfall::{
    ManagementFeeType, WaterfallBuilder,
};
use finstack_valuations::instruments::structured_credit::{components::tranches::TrancheStructure, enums::{DealType, TrancheSeniority}};
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

#[test]
fn pool_stats_wac_was_wam_and_defaults() {
    let mut pool = AssetPool::new("POOL", DealType::CLO, Currency::USD);
    // Two assets: one fixed, one floating with explicit spread
    let a1 = PoolAsset::fixed_rate_bond("B1", Money::new(5_000_000.0, Currency::USD), 0.07, d(2029, 1, 1));
    let a2 = PoolAsset::floating_rate_loan("L1", Money::new(5_000_000.0, Currency::USD), "SOFR-3M", 450.0, d(2030, 1, 1));
    pool.assets.push(a1);
    pool.assets.push(a2);

    // Mark a small default
    let mut defaulted = PoolAsset::fixed_rate_bond("B2", Money::new(1_000_000.0, Currency::USD), 0.08, d(2031, 1, 1));
    defaulted.default_with_recovery(Money::new(200_000.0, Currency::USD), d(2026, 1, 1));
    pool.assets.push(defaulted);

    let stats = calculate_pool_stats(&pool, d(2025, 1, 1));
    // WAC uses asset.rate; WAS uses spread_bps (falls back to rate for fixed)
    assert!(stats.weighted_avg_coupon > 0.0);
    assert!(stats.weighted_avg_spread > 0.0);
    assert!(stats.weighted_avg_maturity > 0.0);
    // Default rate should reflect defaulted balance / total
    assert!(stats.cumulative_default_rate > 0.0);
}

#[test]
fn simple_waterfall_distribution_with_diversion() {
    // Build a minimal tranche structure: A senior and equity
    let mut tranches = TrancheStructure { tranches: vec![] };
    tranches.add_tranche("A", TrancheSeniority::Senior, 1, Money::new(8_000_000.0, Currency::USD), 0.06);
    tranches.add_tranche("EQUITY", TrancheSeniority::Equity, 99, Money::new(2_000_000.0, Currency::USD), 0.0);

    // Build a waterfall with fees, tranche interest/principal, and equity residual
    let wf = WaterfallBuilder::new(Currency::USD)
        .add_senior_expenses(Money::new(25_000.0, Currency::USD), "Trustee")
        .add_management_fee(0.01, ManagementFeeType::Senior)
        .add_tranche_interest("A", true)
        .add_tranche_principal("A")
        .add_equity_distribution()
        .add_oc_ic_trigger("A", Some(1.20), Some(1.10))
        .build();

    // Apply with available cash and interest collections
    let mut engine = wf.clone();
    let available = Money::new(500_000.0, Currency::USD);
    let interest = Money::new(100_000.0, Currency::USD);
    let result = engine
        .apply_waterfall(
            available,
            interest,
            d(2025, 4, 1),
            &tranches,
            Money::new(10_000_000.0, Currency::USD),
            &AssetPool::new("POOL", DealType::CLO, Currency::USD),
            &MarketContext::new(),
        )
        .unwrap();

    // Distributions exist and residual or tranche received some cash
    assert!(result.distributions.values().map(|m| m.amount()).sum::<f64>() > 0.0);
}


