#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::equity_option::parameters::EquityOptionParams;
use finstack_valuations::instruments::{EquityOption, ExerciseStyle, OptionType};
use time::Month;

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


