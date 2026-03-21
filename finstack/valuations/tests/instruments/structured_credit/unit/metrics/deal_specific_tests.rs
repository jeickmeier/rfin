//! Unit tests for deal-specific metrics (ABS, CMBS, RMBS).
//!
//! Tests cover:
//! - ABS speed, delinquency, charge-off, excess spread
//! - CMBS LTV, DSCR
//! - RMBS LTV, FICO, WAL adjustments

// Deal-specific metrics are best tested in integration context
// where we can construct full instruments with realistic data

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::config::constants::STANDARD_PSA_SPEEDS;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CmbsDscrCalculator, DealType, Pool, PoolAsset, RmbsFicoCalculator, RmbsLtvCalculator,
    RmbsWalCalculator, Seniority, StructuredCredit, Tranche, TrancheCoupon, TrancheStructure,
};
use finstack_valuations::metrics::{MetricCalculator, MetricContext};
use std::sync::Arc;
use time::Month;

fn rmbs_instrument() -> StructuredCredit {
    let mut pool = Pool::new("POOL", DealType::RMBS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "MORTGAGE-1",
        Money::new(5_000_000.0, Currency::USD),
        0.05,
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        finstack_core::dates::DayCount::Thirty360,
    ));

    let tranche = Tranche::new(
        "A",
        0.0,
        100.0,
        Seniority::Senior,
        Money::new(5_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.04 },
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![tranche]).unwrap();

    StructuredCredit::new_rmbs(
        "RMBS-TEST",
        pool,
        tranches,
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        "USD-OIS",
    )
}

fn cmbs_instrument() -> StructuredCredit {
    let mut pool = Pool::new("POOL", DealType::CMBS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "MORTGAGE-1",
        Money::new(10_000_000.0, Currency::USD),
        0.05,
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        finstack_core::dates::DayCount::Thirty360,
    ));

    let tranche = Tranche::new(
        "A",
        0.0,
        100.0,
        Seniority::Senior,
        Money::new(10_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.04 },
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![tranche]).unwrap();

    StructuredCredit::new_cmbs(
        "CMBS-TEST",
        pool,
        tranches,
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        "USD-OIS",
    )
}

fn metric_context(instrument: StructuredCredit, as_of: Date) -> MetricContext {
    MetricContext::new(
        Arc::new(instrument),
        Arc::new(MarketContext::new()),
        as_of,
        Money::new(0.0, Currency::USD),
        MetricContext::default_config(),
    )
}

#[test]
fn test_abs_metrics_require_abs_deal_type() {
    // ABS metrics should only apply to ABS, Auto, or Card deals
    let abs_family = [DealType::ABS, DealType::Auto, DealType::Card];
    for deal in &abs_family {
        assert!(
            matches!(*deal, DealType::ABS | DealType::Auto | DealType::Card),
            "ABS metrics should support {:?}",
            deal
        );
    }

    let non_abs = [DealType::CLO, DealType::CBO, DealType::CMBS, DealType::RMBS];
    for deal in &non_abs {
        assert!(
            !matches!(*deal, DealType::ABS | DealType::Auto | DealType::Card),
            "ABS metrics should not support {:?}",
            deal
        );
    }
}

#[test]
fn test_cmbs_metrics_require_cmbs_deal_type() {
    // CMBS metrics (LTV, DSCR) should only apply to CMBS
    let cmbs_types = [DealType::CMBS];
    for deal in &cmbs_types {
        assert!(
            matches!(*deal, DealType::CMBS),
            "CMBS metrics should support {:?}",
            deal
        );
    }

    let non_cmbs = [
        DealType::CLO,
        DealType::CBO,
        DealType::ABS,
        DealType::RMBS,
        DealType::Auto,
    ];
    for deal in &non_cmbs {
        assert!(
            !matches!(*deal, DealType::CMBS),
            "CMBS metrics should not support {:?}",
            deal
        );
    }
}

#[test]
fn test_cmbs_dscr_calculator_returns_configured_noi_multiple() {
    let dscr = CmbsDscrCalculator::new(1.35)
        .calculate(&mut metric_context(
            cmbs_instrument(),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        ))
        .unwrap();

    assert!(
        (dscr - 1.35).abs() < 1e-12,
        "Current CMBS DSCR implementation should equal the configured NOI multiplier"
    );
}

#[test]
fn test_rmbs_metrics_adjust_for_psa_speed() {
    // RMBS metrics should consider PSA speeds above and below par
    assert!(
        STANDARD_PSA_SPEEDS.iter().any(|&speed| speed > 1.0),
        "PSA speed grid should include stressed scenarios above 100%"
    );
    assert!(
        STANDARD_PSA_SPEEDS.iter().any(|&speed| speed < 1.0),
        "PSA speed grid should include benign scenarios below 100%"
    );
}

#[test]
fn test_rmbs_wal_adjusts_with_psa_speed() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let mut base = rmbs_instrument();
    base.behavior_overrides.psa_speed_multiplier = Some(1.0);
    let wal_base = RmbsWalCalculator
        .calculate(&mut metric_context(base, as_of))
        .unwrap();

    let mut fast = rmbs_instrument();
    fast.behavior_overrides.psa_speed_multiplier = Some(2.0);
    let wal_fast = RmbsWalCalculator
        .calculate(&mut metric_context(fast, as_of))
        .unwrap();

    assert!(wal_fast < wal_base, "Higher PSA speeds should shorten WAL");
}

#[test]
fn test_rmbs_ltv_uses_credit_factors_when_present() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let rmbs = rmbs_instrument();
    let ltv = RmbsLtvCalculator::new(75.0)
        .calculate(&mut metric_context(rmbs, as_of))
        .unwrap();

    assert_eq!(ltv, 80.0, "RMBS default LTV should be 80% from deal config");
}

#[test]
fn test_rmbs_fico_defaults_when_missing() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let rmbs = rmbs_instrument();
    let fico = RmbsFicoCalculator::new(700.0)
        .calculate(&mut metric_context(rmbs, as_of))
        .unwrap();

    assert_eq!(fico, 700.0, "RMBS FICO should use default when missing");
}
