//! Construction tests for interest rate options.
//!
//! Validates instrument creation, parameter handling, and builder patterns.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_valuations::instruments::rates::cap_floor::InterestRateOptionParams;
use finstack_valuations::instruments::rates::cap_floor::{InterestRateOption, RateOptionType};
use finstack_valuations::instruments::{ExerciseStyle, SettlementType};
use rust_decimal::Decimal;
use time::Month;

#[test]
fn test_cap_creation_basic() {
    let notional = Money::new(10_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let params =
        InterestRateOptionParams::cap(notional, 0.03, Tenor::quarterly(), DayCount::Act360);
    let cap = InterestRateOption::new(
        "USD_CAP_3%",
        &params,
        start,
        end,
        "USD-OIS",
        "USD-LIBOR-3M",
        "USD-CAP-VOL",
    );

    assert_eq!(cap.id, "USD_CAP_3%".into());
    assert_eq!(cap.rate_option_type, RateOptionType::Cap);
    assert_eq!(cap.notional.amount(), 10_000_000.0);
    assert_eq!(cap.notional.currency(), Currency::USD);
    assert_eq!(
        cap.strike_rate,
        Decimal::try_from(0.03).expect("valid decimal")
    );
    assert_eq!(cap.frequency, Tenor::quarterly());
    assert_eq!(cap.day_count, DayCount::Act360);
    assert_eq!(cap.start_date, start);
    assert_eq!(cap.maturity, end);
}

#[test]
fn test_floor_creation_basic() {
    let notional = Money::new(5_000_000.0, Currency::EUR);
    let start = Date::from_calendar_date(2025, Month::March, 15).unwrap();
    let end = Date::from_calendar_date(2028, Month::March, 15).unwrap();

    let params =
        InterestRateOptionParams::floor(notional, 0.01, Tenor::semi_annual(), DayCount::Thirty360);
    let floor = InterestRateOption::new(
        "EUR_FLOOR_1%",
        &params,
        start,
        end,
        "EUR-OIS",
        "EUR-EURIBOR-6M",
        "EUR-CAP-VOL",
    );

    assert_eq!(floor.id, "EUR_FLOOR_1%".into());
    assert_eq!(floor.rate_option_type, RateOptionType::Floor);
    assert_eq!(floor.notional.amount(), 5_000_000.0);
    assert_eq!(floor.notional.currency(), Currency::EUR);
    assert_eq!(
        floor.strike_rate,
        Decimal::try_from(0.01).expect("valid decimal")
    );
    assert_eq!(floor.frequency, Tenor::semi_annual());
}

#[test]
fn test_cap_new_cap_helper() {
    let notional = Money::new(1_000_000.0, Currency::GBP);
    let start = Date::from_calendar_date(2025, Month::June, 1).unwrap();
    let end = Date::from_calendar_date(2027, Month::June, 1).unwrap();

    let cap = InterestRateOption::new_cap(
        "GBP_CAP",
        notional,
        0.04,
        start,
        end,
        Tenor::quarterly(),
        DayCount::Act365F,
        "GBP-OIS",
        "GBP-LIBOR-3M",
        "GBP-CAP-VOL",
    );

    assert_eq!(cap.rate_option_type, RateOptionType::Cap);
    assert_eq!(
        cap.strike_rate,
        Decimal::try_from(0.04).expect("valid decimal")
    );
    assert_eq!(cap.notional.currency(), Currency::GBP);
}

#[test]
fn test_floor_new_floor_helper() {
    let notional = Money::new(2_000_000.0, Currency::JPY);
    let start = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2031, Month::January, 1).unwrap();

    let floor = InterestRateOption::new_floor(
        "JPY_FLOOR",
        notional,
        0.005,
        start,
        end,
        Tenor::quarterly(),
        DayCount::Act360,
        "JPY-OIS",
        "JPY-LIBOR-3M",
        "JPY-CAP-VOL",
    );

    assert_eq!(floor.rate_option_type, RateOptionType::Floor);
    assert_eq!(
        floor.strike_rate,
        Decimal::try_from(0.005).expect("valid decimal")
    );
}

#[test]
fn test_caplet_creation() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();

    let caplet = InterestRateOption {
        id: "CAPLET_TEST".into(),
        rate_option_type: RateOptionType::Caplet,
        notional,
        strike_rate: Decimal::try_from(0.05).expect("valid decimal"),
        start_date: start,
        maturity: end,
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        stub: StubKind::None,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD_OIS".into(),
        forward_curve_id: "USD_LIBOR_3M".into(),
        vol_surface_id: "USD_CAP_VOL".into(),
        vol_type: Default::default(),

        pricing_overrides: finstack_valuations::instruments::PricingOverrides::default(),
        attributes: Default::default(),
    };

    assert_eq!(caplet.rate_option_type, RateOptionType::Caplet);
    assert!(end > start);
}

#[test]
fn test_floorlet_creation() {
    let notional = Money::new(500_000.0, Currency::EUR);
    let start = Date::from_calendar_date(2025, Month::March, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::September, 1).unwrap();

    let floorlet = InterestRateOption {
        id: "FLOORLET_TEST".into(),
        rate_option_type: RateOptionType::Floorlet,
        notional,
        strike_rate: Decimal::try_from(0.02).expect("valid decimal"),
        start_date: start,
        maturity: end,
        frequency: Tenor::semi_annual(),
        day_count: DayCount::Act360,
        stub: StubKind::None,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        exercise_style: ExerciseStyle::European,
        settlement: SettlementType::Cash,
        discount_curve_id: "EUR_OIS".into(),
        forward_curve_id: "EUR_EURIBOR_6M".into(),
        vol_surface_id: "EUR_CAP_VOL".into(),
        vol_type: Default::default(),

        pricing_overrides: finstack_valuations::instruments::PricingOverrides::default(),
        attributes: Default::default(),
    };

    assert_eq!(floorlet.rate_option_type, RateOptionType::Floorlet);
}

#[test]
fn test_custom_calendar() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let params =
        InterestRateOptionParams::cap(notional, 0.03, Tenor::quarterly(), DayCount::Act360);
    let mut cap = InterestRateOption::new(
        "CAP_WITH_CALENDAR",
        &params,
        start,
        end,
        "USD-OIS",
        "USD-LIBOR-3M",
        "USD-CAP-VOL",
    );
    cap.calendar_id = Some("US_NERC".into());

    assert_eq!(cap.calendar_id.as_deref(), Some("US_NERC"));
}

#[test]
fn test_different_day_counts() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let day_counts = vec![
        DayCount::Act360,
        DayCount::Act365F,
        DayCount::Thirty360,
        DayCount::ActActIsma,
    ];

    for dc in day_counts {
        let params = InterestRateOptionParams::cap(notional, 0.03, Tenor::quarterly(), dc);
        let cap = InterestRateOption::new(
            "CAP_DC_TEST",
            &params,
            start,
            end,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-CAP-VOL",
        );

        assert_eq!(cap.day_count, dc);
    }
}

#[test]
fn test_different_frequencies() {
    let notional = Money::new(1_000_000.0, Currency::USD);
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let frequencies = vec![
        Tenor::monthly(),
        Tenor::quarterly(),
        Tenor::semi_annual(),
        Tenor::annual(),
    ];

    for freq in frequencies {
        let params = InterestRateOptionParams::cap(notional, 0.03, freq, DayCount::Act360);
        let cap = InterestRateOption::new(
            "CAP_FREQ_TEST",
            &params,
            start,
            end,
            "USD-OIS",
            "USD-LIBOR-3M",
            "USD-CAP-VOL",
        );

        assert_eq!(cap.frequency, freq);
    }
}
