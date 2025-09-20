//! Integration tests for new derivative instruments

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::cap_floor::parameters::InterestRateOptionParams;
#[allow(unused_imports)]
use finstack_valuations::instruments::cap_floor::InterestRateOption;
use finstack_valuations::instruments::cds::CDSConvention;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::equity_option::parameters::EquityOptionParams;
use finstack_valuations::instruments::fx_option::parameters::FxOptionParams;
use finstack_valuations::instruments::inflation_linked_bond::parameters::InflationLinkedBondParams;
use finstack_valuations::instruments::inflation_linked_bond::IndexationMethod;
use finstack_valuations::instruments::{
    CreditDefaultSwap, CdsOption, EquityOption, FxOption, InflationLinkedBond,
};
use finstack_valuations::instruments::{ExerciseStyle, OptionType};
use time::Month;

#[test]
fn test_cds_creation_and_basic_pricing() {
    // Create a CDS instrument
    let notional = Money::new(10_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let spread_bp = 100.0; // 100bp
    // DateRange inlined; start/end passed directly to constructor
    let cds = CreditDefaultSwap::new_isda(
        "CDS_TEST",
        notional,
        finstack_valuations::instruments::cds::PayReceive::PayProtection,
        finstack_valuations::instruments::cds::CDSConvention::IsdaNa,
        spread_bp,
        start,
        end,
        "ABC Corp",
        0.4,
        "USD-OIS",
        "ABC-SENIOR",
    );

    assert_eq!(cds.id, "CDS_TEST");
    assert_eq!(cds.reference_entity, "ABC Corp");
    assert_eq!(cds.premium.spread_bp, 100.0);
    assert_eq!(cds.protection.recovery_rate, 0.4);
    assert_eq!(cds.convention, CDSConvention::IsdaNa);
}

#[test]
fn test_equity_option_creation() {
    let strike = Money::new(100.0, Currency::USD);
    let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

    let option_params = EquityOptionParams::european_call(
        strike, expiry, 100.0, // Contract size
    );
    let underlying_params =
        finstack_valuations::instruments::underlying::EquityUnderlyingParams::new(
            "AAPL",
            "AAPL-SPOT",
        );
    let option = EquityOption::new(
        "AAPL_CALL_100",
        &option_params,
        &underlying_params,
        CurveId::new("USD-OIS"),
        CurveId::new("AAPL-VOL"),
    );

    assert_eq!(option.id, "AAPL_CALL_100".into());
    assert_eq!(option.underlying_ticker, "AAPL");
    assert_eq!(option.strike.amount(), 100.0);
    assert_eq!(option.option_type, OptionType::Call);
    assert_eq!(option.exercise_style, ExerciseStyle::European);

    // Test Black-Scholes pricing
    let spot = 110.0;
    let r = 0.05;
    let sigma = 0.25;
    let t = 1.0;
    let q = 0.02;

    let price = option.black_scholes_price(spot, r, sigma, t, q).unwrap();
    assert!(price.amount() > 0.0); // Call should have positive value when spot > strike

    // Test Greeks
    let delta = option.delta(spot, r, sigma, t, q);
    assert!(delta > 0.0 && delta < 1.0); // Call delta should be between 0 and 1

    let gamma = option.gamma(spot, r, sigma, t, q);
    assert!(gamma > 0.0); // Gamma should be positive
}

#[test]
fn test_fx_option_creation() {
    let notional = Money::new(1_000_000.0, Currency::EUR);
    let expiry = Date::from_calendar_date(2025, Month::December, 31).unwrap();

    let option_params = FxOptionParams::european_call(1.20, expiry, notional);
    let underlying_params = finstack_valuations::instruments::fx_option::FxUnderlyingParams::new(
        Currency::EUR,
        Currency::USD,
        "USD-OIS",
        "EUR-OIS",
    );

    let option = FxOption::new(
        "EURUSD_CALL_1.20",
        &option_params,
        &underlying_params,
        "EURUSD-VOL",
    );

    assert_eq!(option.id, "EURUSD_CALL_1.20");
    assert_eq!(option.base_currency, Currency::EUR);
    assert_eq!(option.quote_currency, Currency::USD);
    assert_eq!(option.strike, 1.20);

    // Test Garman-Kohlhagen pricing
    let spot = 1.25;
    let r_d = 0.05; // USD rate
    let r_f = 0.03; // EUR rate
    let sigma = 0.10;
    let t = 1.0;

    let price = option
        .garman_kohlhagen_price(spot, r_d, r_f, sigma, t)
        .unwrap();
    assert!(price.amount() > 0.0); // Call should have positive value when spot > strike
    assert_eq!(price.currency(), Currency::USD);
}

#[test]
fn test_interest_rate_option_creation() {
    let notional = Money::new(10_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    use finstack_valuations::instruments::cap_floor::InterestRateOption;
    let params =
        InterestRateOptionParams::cap(notional, 0.03, Frequency::quarterly(), DayCount::Act360);
    let cap = InterestRateOption::new(
        "USD_CAP_3%",
        &params,
        start,
        end,
        "USD-OIS",
        "USD-LIBOR-3M",
        "USD-CAP-VOL",
    );

    assert_eq!(cap.id, "USD_CAP_3%");
    assert_eq!(cap.strike_rate, 0.03);
    assert_eq!(cap.frequency, Frequency::quarterly());
}

#[test]
fn test_credit_option_creation() {
    let notional = Money::new(10_000_000.0, Currency::USD);
    let expiry = Date::from_calendar_date(2025, Month::June, 30).unwrap();
    let cds_maturity = Date::from_calendar_date(2030, Month::June, 30).unwrap();

    let option_params = CdsOptionParams::call(
        200.0, // 200bp strike
        expiry,
        cds_maturity,
        notional,
    );
    let credit_params = finstack_valuations::instruments::CreditParams::new(
        "ABC Corp",
        0.4, // 40% recovery
        "ABC-SENIOR",
    );
    let option = CdsOption::new(
        "ABC_CDS_CALL_200",
        &option_params,
        &credit_params,
        "USD-OIS",
        "ABC-SENIOR",
        "ABC-CDS-VOL",
    );

    assert_eq!(option.id, "ABC_CDS_CALL_200");
    assert_eq!(option.reference_entity, "ABC Corp");
    assert_eq!(option.strike_spread_bp, 200.0);
    assert_eq!(option.recovery_rate, 0.4);
}

#[test]
fn test_inflation_linked_bond_creation() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let issue = Date::from_calendar_date(2020, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    let bond_params = InflationLinkedBondParams::tips(
        notional, 0.0125, // 1.25% real coupon
        issue, maturity, 250.0, // Base CPI
    );

    let tips = InflationLinkedBond::new_tips("US_TIPS_2030", &bond_params, "USD-REAL", "US-CPI-U");

    assert_eq!(tips.id, "US_TIPS_2030");
    assert_eq!(tips.indexation_method, IndexationMethod::TIPS);
    assert_eq!(tips.real_coupon, 0.0125);
    assert_eq!(tips.base_index, 250.0);

    // Test UK linker creation
    let gbp_notional = Money::new(1_000_000.0, Currency::GBP);
    let base_date = Date::from_calendar_date(2019, Month::November, 1).unwrap();

    let uk_bond_params = InflationLinkedBondParams::uk_linker(
        gbp_notional,
        0.00625, // 0.625% real coupon
        issue,
        maturity,
        280.0, // Base RPI
    );

    let uk_linker = InflationLinkedBond::new_uk_linker(
        "UK_LINKER_2040",
        &uk_bond_params,
        base_date,
        "GBP-NOMINAL",
        "UK-RPI",
    );

    assert_eq!(uk_linker.id, "UK_LINKER_2040");
    assert_eq!(uk_linker.indexation_method, IndexationMethod::UK);
}

#[test]
fn test_cds_isda_conventions() {
    // Test different ISDA conventions
    assert_eq!(CDSConvention::IsdaNa.day_count(), DayCount::Act360);
    assert_eq!(CDSConvention::IsdaEu.day_count(), DayCount::Act360);
    assert_eq!(CDSConvention::IsdaAs.day_count(), DayCount::Act365F);

    assert_eq!(CDSConvention::IsdaNa.frequency(), Frequency::quarterly());
    assert_eq!(CDSConvention::IsdaEu.frequency(), Frequency::quarterly());
}
