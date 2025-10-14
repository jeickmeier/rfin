#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_valuations::instruments::cap_floor::parameters::InterestRateOptionParams;
use time::Month;

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

    assert_eq!(cap.id, "USD_CAP_3%".into());
    assert_eq!(cap.strike_rate, 0.03);
    assert_eq!(cap.frequency, Frequency::quarterly());
}


