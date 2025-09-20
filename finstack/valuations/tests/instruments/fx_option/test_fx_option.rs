#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::fx_option::parameters::FxOptionParams;
use finstack_valuations::instruments::FxOption;
use time::Month;

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


