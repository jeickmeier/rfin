//! Tests for structured credit constructors and behavioral overrides.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor};
use finstack_core::money::Money;
use finstack_valuations::constants::DECIMAL_TO_PERCENT;
use finstack_valuations::instruments::structured_credit::types::constants::{
    ABS_AUTO_STANDARD_CDR, CLO_STANDARD_CDR, CMBS_STANDARD_CDR, PSA_RAMP_MONTHS, PSA_TERMINAL_CPR,
    RMBS_STANDARD_CDR, SDA_PEAK_CDR, SDA_PEAK_MONTH, SDA_TERMINAL_CDR,
};
use finstack_valuations::instruments::structured_credit::utils::rates::{cdr_to_mdr, cpr_to_smm};
use finstack_valuations::instruments::structured_credit::{
    DealType, Pool, PoolAsset, StructuredCredit, Tranche, TrancheCoupon, TrancheStructure,
    Waterfall,
};
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

fn create_pool_with_balance(balance: f64) -> Pool {
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

fn create_single_tranche() -> TrancheStructure {
    let tranche = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        finstack_valuations::instruments::structured_credit::Seniority::Senior,
        Money::new(1_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .unwrap();
    TrancheStructure::new(vec![tranche]).unwrap()
}

fn create_waterfall(tranches: &TrancheStructure) -> Waterfall {
    Waterfall::standard_sequential(Currency::USD, tranches, vec![])
}

#[test]
fn test_apply_deal_defaults_sets_expected_assumptions() {
    let pool = create_pool_with_balance(1_000_000.0);
    let tranches = create_single_tranche();
    let waterfall = create_waterfall(&tranches);
    let closing = Date::from_calendar_date(2024, Month::January, 1).unwrap();
    let legal = maturity_date();

    let cases = [
        (DealType::ABS, Tenor::monthly(), ABS_AUTO_STANDARD_CDR),
        (DealType::CLO, Tenor::quarterly(), CLO_STANDARD_CDR),
        (DealType::CMBS, Tenor::monthly(), CMBS_STANDARD_CDR),
        (DealType::RMBS, Tenor::monthly(), RMBS_STANDARD_CDR),
    ];

    for (deal_type, expected_frequency, expected_cdr) in cases {
        let sc = StructuredCredit::apply_deal_defaults(
            format!("TEST-{deal_type:?}"),
            deal_type,
            pool.clone(),
            tranches.clone(),
            waterfall.clone(),
            closing,
            legal,
            "USD-OIS",
        );

        assert_eq!(sc.deal_type, deal_type);
        assert_eq!(sc.payment_frequency, expected_frequency);
        assert!((sc.default_assumptions.base_cdr_annual - expected_cdr).abs() < 1e-12);
    }
}

#[test]
fn test_example_has_expected_defaults() {
    let sc = StructuredCredit::example();
    let waterfall = sc.create_waterfall();

    assert_eq!(sc.tranches.tranches.len(), 1);
    assert_eq!(waterfall.tiers.len(), 3);
    assert_eq!(sc.payment_calendar_id.as_deref(), Some("nyse"));
}

#[test]
fn test_prepayment_overrides_use_expected_priority() {
    let pool = create_pool_with_balance(1_000_000.0);
    let tranches = create_single_tranche();
    let waterfall = create_waterfall(&tranches);
    let mut sc = StructuredCredit::new_abs(
        "TEST-ABS",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    sc.behavior_overrides.abs_speed = Some(0.02);
    let abs_rate = sc.calculate_prepayment_rate(test_date(), 1);
    assert!((abs_rate - 0.02).abs() < 1e-12);

    sc.behavior_overrides.abs_speed = None;
    sc.behavior_overrides.cpr_annual = Some(0.12);
    let cpr_rate = sc.calculate_prepayment_rate(test_date(), 1);
    assert!((cpr_rate - cpr_to_smm(0.12)).abs() < 1e-12);

    sc.behavior_overrides.cpr_annual = None;
    sc.behavior_overrides.psa_speed_multiplier = Some(2.0);
    let seasoning = 3;
    let base_cpr = (seasoning as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR;
    let expected = cpr_to_smm(base_cpr * 2.0);
    let psa_rate = sc.calculate_prepayment_rate(test_date(), seasoning);
    assert!((psa_rate - expected).abs() < 1e-12);
}

#[test]
fn test_default_overrides_use_expected_priority() {
    let pool = create_pool_with_balance(1_000_000.0);
    let tranches = create_single_tranche();
    let waterfall = create_waterfall(&tranches);
    let mut sc = StructuredCredit::new_abs(
        "TEST-ABS",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );

    sc.behavior_overrides.cdr_annual = Some(0.12);
    let cdr_rate = sc.calculate_default_rate(test_date(), 1);
    assert!((cdr_rate - cdr_to_mdr(0.12)).abs() < 1e-12);

    sc.behavior_overrides.cdr_annual = None;
    sc.behavior_overrides.sda_speed_multiplier = Some(1.5);
    let seasoning = SDA_PEAK_MONTH + 1;
    let decline_period = (SDA_PEAK_MONTH * 2 - SDA_PEAK_MONTH) as f64;
    let months_past_peak = (seasoning - SDA_PEAK_MONTH) as f64;
    let cdr = (SDA_PEAK_CDR
        - (months_past_peak / decline_period) * (SDA_PEAK_CDR - SDA_TERMINAL_CDR))
        * 1.5;
    let expected = 1.0 - (1.0 - cdr).powf(1.0 / 12.0);
    let sda_rate = sc.calculate_default_rate(test_date(), seasoning);
    assert!((sda_rate - expected).abs() < 1e-12);
}

#[test]
fn test_current_loss_percentage_handles_zero_balance_and_offsets() {
    let empty_pool = create_pool_with_balance(0.0);
    let tranches = create_single_tranche();
    let waterfall = create_waterfall(&tranches);
    let sc_zero = StructuredCredit::new_abs(
        "TEST-ZERO",
        empty_pool,
        tranches.clone(),
        waterfall.clone(),
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );
    assert_eq!(sc_zero.current_loss_percentage().unwrap(), 0.0);

    let mut pool = create_pool_with_balance(1_000_000.0);
    pool.cumulative_defaults = Money::new(50_000.0, Currency::USD);
    pool.cumulative_recoveries = Money::new(10_000.0, Currency::USD);
    let sc = StructuredCredit::new_abs(
        "TEST-LOSS",
        pool,
        tranches,
        waterfall,
        Date::from_calendar_date(2024, Month::January, 1).unwrap(),
        maturity_date(),
        "USD-OIS",
    );
    let expected = (50_000.0 - 10_000.0) / 1_000_000.0 * DECIMAL_TO_PERCENT;
    let actual = sc.current_loss_percentage().unwrap();
    assert!((actual - expected).abs() < 1e-12);
}
