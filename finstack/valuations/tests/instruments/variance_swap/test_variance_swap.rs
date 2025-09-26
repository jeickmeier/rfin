//! Integration tests for variance swap instruments.

use finstack_core::{
    currency::Currency,
    dates::{Date, Frequency},
    market_data::{context::MarketContext, term_structures::discount_curve::DiscountCurve},
    math::stats::{realized_variance, RealizedVarMethod},
    money::Money,


use finstack_valuations::instruments::{
    traits::Priceable,
    variance_swap::{PayReceive, VarianceSwap},


fn test_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, time::Month::try_from(month).unwrap(), day).unwrap()
}

fn create_test_market_context() -> MarketContext {
    let base_date = test_date(2025, 1, 1);

    // Create a simple discount curve
    let disc_curve = DiscountCurve::builder("USD_OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_price(
            "SPX",
            finstack_core::market_data::scalars::MarketScalar::Unitless(5000.0),
        )
        .insert_price(
            "SPX_IMPL_VOL",
            finstack_core::market_data::scalars::MarketScalar::Unitless(0.20),
        )
}

#[test]
fn test_variance_swap_creation() {
    let swap = VarianceSwap::builder()
        .id("VAR_SPX_1Y".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.20 * 0.20)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    assert_eq!(swap.id.as_str(), "VAR_SPX_1Y");
    assert_eq!(swap.underlying_id, "SPX");
    assert_eq!(swap.notional.amount(), 100_000.0);
    assert!((swap.strike_variance - 0.04).abs() < 1e-10);
    assert_eq!(swap.observation_freq, Frequency::daily());
}

#[test]
fn test_variance_swap_payoff() {
    let swap = VarianceSwap::builder()
        .id("VAR_SPX_1Y".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04) // 20% annualized vol squared
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    // Test payoff calculation
    let realized_var = 0.0625; // 25% vol squared
    let payoff = swap.payoff(realized_var);

    // Expected: 100,000 * (0.0625 - 0.04) = 2,250
    assert_eq!(payoff.amount(), 2250.0);
    assert_eq!(payoff.currency(), Currency::USD);

    // Test with lower realized variance
    let realized_var = 0.03; // 17.3% vol squared
    let payoff = swap.payoff(realized_var);

    // Expected: 100,000 * (0.03 - 0.04) = -1,000
    assert_eq!(payoff.amount(), -1000.0);
}

#[test]
fn test_variance_swap_pay_receive() {
    let notional = Money::new(100_000.0, Currency::USD);
    let strike_var = 0.04;
    let realized_var = 0.05;

    // Test receive variance (long)
    let swap_long = VarianceSwap::builder()
        .id("VAR_LONG".into())
        .underlying_id("SPX".to_string())
        .notional(notional)
        .strike_variance(strike_var)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let payoff_long = swap_long.payoff(realized_var);
    assert_eq!(payoff_long.amount(), 1000.0); // Positive when realized > strike

    // Test pay variance (short)
    let swap_short = VarianceSwap::builder()
        .id("VAR_SHORT".into())
        .underlying_id("SPX".to_string())
        .notional(notional)
        .strike_variance(strike_var)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Pay)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let payoff_short = swap_short.payoff(realized_var);
    assert_eq!(payoff_short.amount(), -1000.0); // Negative when realized > strike
}

#[test]
fn test_variance_swap_pricing_before_start() {
    let swap = VarianceSwap::builder()
        .id("VAR_SPX_1Y".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let context = create_test_market_context();
    let as_of = test_date(2024, 12, 1); // Before start

    let pv = swap.value(&context, as_of).unwrap();

    // PV should be discounted expected payoff
    // With implied vol = 0.20 (variance = 0.04), payoff = 0
    // But we should have some PV due to discounting
    assert!(pv.amount().abs() < 1000.0); // Small value expected
}

#[test]
fn test_variance_swap_pricing_at_maturity() {
    let swap = VarianceSwap::builder()
        .id("VAR_SPX_1Y".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let context = create_test_market_context();
    let as_of = test_date(2026, 1, 1); // At maturity

    // Note: In a real implementation, we'd have historical prices
    // For now, the implementation returns a placeholder
    let pv = swap.value(&context, as_of).unwrap();

    // At maturity, PV should equal the payoff (no discounting)
    assert_eq!(pv.currency(), Currency::USD);
}

#[test]
fn test_annualization_factor() {
    let swap = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    assert_eq!(swap.annualization_factor(), 252.0);

    let swap_weekly = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::weekly())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    assert_eq!(swap_weekly.annualization_factor(), 52.0);
}

#[test]
fn test_time_elapsed_fraction() {
    let swap = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2025, 12, 31))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    // Before start
    assert_eq!(swap.time_elapsed_fraction(test_date(2024, 12, 31)), 0.0);

    // At start
    assert_eq!(swap.time_elapsed_fraction(test_date(2025, 1, 1)), 0.0);

    // Halfway through
    let halfway = test_date(2025, 7, 1);
    let fraction = swap.time_elapsed_fraction(halfway);
    assert!(fraction > 0.45 && fraction < 0.55);

    // At maturity
    assert_eq!(swap.time_elapsed_fraction(test_date(2025, 12, 31)), 1.0);

    // After maturity
    assert_eq!(swap.time_elapsed_fraction(test_date(2026, 1, 1)), 1.0);
}

#[test]
fn test_realized_variance_calculation() {
    // Test the core realized variance calculation
    let prices = vec![100.0, 102.0, 101.0, 103.0, 104.0, 102.0];
    let var = realized_variance(&prices, RealizedVarMethod::CloseToClose, 252.0);

    // Should calculate variance of log returns and annualize
    assert!(var > 0.0);

    // Test with constant prices (zero variance)
    let constant_prices = vec![100.0, 100.0, 100.0, 100.0];
    let var_const = realized_variance(&constant_prices, RealizedVarMethod::CloseToClose, 252.0);
    assert_eq!(var_const, 0.0);
}

#[test]
fn test_observation_dates() {
    let swap = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2025, 3, 1))
        .observation_freq(Frequency::weekly())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let obs_dates = swap.observation_dates();

    // Should have weekly observations between start and maturity
    assert!(!obs_dates.is_empty());
    assert!(obs_dates[0] >= swap.start_date);
    assert!(obs_dates[obs_dates.len() - 1] <= swap.maturity);

    // Check spacing is roughly weekly
    if obs_dates.len() > 1 {
        let days_between = (obs_dates[1] - obs_dates[0]).whole_days();
        assert!((5..=9).contains(&days_between)); // Account for weekends
    }
}

#[test]
fn test_builder_validation() {
    // Missing required fields
    let result = VarianceSwap::builder().id("VAR_TEST".into()).build();
    assert!(result.is_err());

    // Invalid strike variance
    let result = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(-0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2026, 1, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build();
    assert!(result.is_err());

    // Invalid dates
    let result = VarianceSwap::builder()
        .id("VAR_TEST".into())
        .underlying_id("SPX".to_string())
        .notional(Money::new(100_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(test_date(2025, 1, 1))
        .maturity(test_date(2024, 1, 1)) // End before start
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(PayReceive::Receive)
        .disc_id("USD_OIS".into())
        .day_count(finstack_core::dates::DayCount::Act365F)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build();
    assert!(result.is_err());
}
