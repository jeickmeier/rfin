//! Comprehensive tests for convertible bond pricing framework.

use crate::instruments::convertible::pricing::{
    calculate_convertible_greeks, calculate_parity, price_convertible_bond, ConvertibleTreeType,
};
use super::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;

use crate::cashflow::builder::types::{CouponType, FixedCouponSpec};
use crate::instruments::bond::{CallPut, CallPutSchedule};
use crate::instruments::models::{
    single_factor_equity_state, BinomialTree, NodeState, TreeModel, TreeValuator, TrinomialTree,
};

use time::Month;

fn create_test_convertible_bond() -> ConvertibleBond {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0), // 10 shares per $1000 bond
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let fixed_coupon = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05, // 5% annual coupon
        freq: Frequency::semi_annual(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    ConvertibleBond {
        id: "TEST_CONVERTIBLE_5Y".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: Some(fixed_coupon),
        floating_coupon: None,
        attributes: Default::default(),
    }
}

fn create_test_market_context() -> MarketContext {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create a flat discount curve at 3% that extends beyond bond maturity
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (10.0, 0.741)]) // ~3% rate: e^(-0.03*10) = 0.741
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price("AAPL", MarketScalar::Unitless(150.0)) // $150 stock price
        .insert_price("AAPL-VOL", MarketScalar::Unitless(0.25)) // 25% volatility
        .insert_price("AAPL-DIVYIELD", MarketScalar::Unitless(0.02)) // 2% dividend yield
}

#[test]
fn test_convertible_bond_parity_calculation() {
    let bond = create_test_convertible_bond();
    let parity = calculate_parity(&bond, 150.0);

    // With 10 shares per $1000 bond and $150 stock price:
    // Conversion value = 10 * 150 = $1,500
    // Parity = $1,500 / $1,000 = 1.5 (150%)
    assert!((parity - 1.5).abs() < 1e-9);
}

#[test]
fn test_convertible_bond_pricing_binomial() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(50)).unwrap();

    // Should be worth at least the conversion value
    let conversion_value = 150.0 * 10.0; // $1,500
    assert!(price.amount() >= conversion_value);

    // Should be in a reasonable range for this scenario
    assert!(price.amount() > 1400.0 && price.amount() < 2000.0);
    assert_eq!(price.currency(), Currency::USD);
}

#[test]
fn test_excludes_reset_events_from_coupon_map() {
    // Build a bond with a floating spec to ensure resets exist; validate pricing runs
    // and does not panic due to treating FloatReset as a coupon cashflow directly.
    use crate::cashflow::builder::types::{FloatingCouponSpec, FloatWindow, FloatCouponParams};

    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let floating = FloatingCouponSpec {
        index_id: "USD-SOFR-3M",
        margin_bp: 0.0,
        window: FloatWindow::from_params(FloatCouponParams {
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            gearing: 1.0,
        }),
    };

    let bond = ConvertibleBond {
        id: "TEST_FLOATING_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: Some(floating),
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();
    let pv = price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(20)).unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_day_count_propagation_for_call_put_mapping() {
    // Ensures we use schedule.day_count (not hardcoded) when mapping call/put dates.
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let call_date = Date::from_calendar_date(2025, Month::July, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let mut call_put = CallPutSchedule::default();
    call_put.calls.push(CallPut { date: call_date, price_pct_of_par: 101.0 });

    let bond = ConvertibleBond {
        id: "TEST_DAYCOUNT_PROP".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: Some(call_put),
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();
    // Should run without errors; internal mapping uses schedule.day_count
    let pv = price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(10)).unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_convertible_bond_pricing_trinomial() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Trinomial(50)).unwrap();

    // Should be worth at least the conversion value
    let conversion_value = 150.0 * 10.0; // $1,500
    assert!(price.amount() >= conversion_value);

    // Should be in a reasonable range
    assert!(price.amount() > 1400.0 && price.amount() < 2000.0);
}

#[test]
fn test_binomial_vs_trinomial_convergence() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();

    let bin_price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(100)).unwrap();

    let tri_price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Trinomial(100))
            .unwrap();

    // Should converge to similar values with sufficient steps
    let diff_pct = (bin_price.amount() - tri_price.amount()).abs() / bin_price.amount();
    assert!(diff_pct < 0.05); // Within 5%
}

#[test]
fn test_convertible_greeks_calculation() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();

    let greeks = calculate_convertible_greeks(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(50),
        Some(0.01),
    );

    assert!(greeks.is_ok());
    let greeks = greeks.unwrap();

    // Delta should be positive (increases with stock price)
    // For deep ITM convertible, delta should approach conversion ratio
    assert!(greeks.delta > 0.0);
    assert!(greeks.delta <= 10.0); // Should not exceed conversion ratio

    // Gamma should be non-negative
    assert!(greeks.gamma >= 0.0);

    // Vega should be positive (higher vol = higher option value)
    assert!(greeks.vega >= 0.0);

    // Price should be reasonable
    assert!(greeks.price > 1400.0);
}

#[test]
fn test_out_of_money_convertible() {
    let bond = create_test_convertible_bond();
    let mut market_context = create_test_market_context();

    // Set a low stock price to make conversion out-of-money
    market_context = market_context.insert_price("AAPL", MarketScalar::Unitless(50.0));

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(50)).unwrap();

    // Should be worth close to bond value (conversion value = 50*10 = 500)
    // Bond should be worth close to its debt value
    assert!(price.amount() < 1200.0); // Less than deep ITM case
    assert!(price.amount() > 800.0); // But more than just conversion value
}

#[test]
fn test_low_volatility_convertible() {
    let bond = create_test_convertible_bond();
    let mut market_context = create_test_market_context();

    // Set low volatility (but not too low to avoid numerical issues)
    market_context = market_context.insert_price("AAPL-VOL", MarketScalar::Unitless(0.05));

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(20)).unwrap();

    // With low vol, should be close to max(bond_value, conversion_value)
    let conversion_value = 150.0 * 10.0; // $1,500
    assert!(price.amount() >= conversion_value * 0.95); // Allow for rounding
}

#[test]
fn test_tree_framework_flexibility() {
    // Test that we can use the generic tree framework directly

    // Simple test valuator that just returns the spot price
    struct SpotReturner;
    impl TreeValuator for SpotReturner {
        fn value_at_maturity(&self, state: &NodeState) -> finstack_core::Result<finstack_core::F> {
            Ok(state.spot().unwrap_or(0.0))
        }

        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: finstack_core::F,
        ) -> finstack_core::Result<finstack_core::F> {
            Ok(continuation_value)
        }
    }

    let market_context = create_test_market_context();
    let initial_vars = single_factor_equity_state(100.0, 0.05, 0.02, 0.20);
    let valuator = SpotReturner;

    // Test both tree types work with the generic interface
    let binomial = BinomialTree::crr(20);
    let price_bin = binomial.price(initial_vars.clone(), 1.0, &market_context, &valuator);
    assert!(price_bin.is_ok());

    let trinomial = TrinomialTree::standard(20);
    let price_tri = trinomial.price(initial_vars, 1.0, &market_context, &valuator);
    assert!(price_tri.is_ok());

    // Both should return approximately the initial spot price for this simple valuator
    assert!((price_bin.unwrap() - 100.0).abs() < 5.0);
    assert!((price_tri.unwrap() - 100.0).abs() < 5.0);
}

#[test]
fn test_mandatory_conversion_policy() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let mandatory_date = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(8.0), // 8 shares per bond
        price: None,
        policy: ConversionPolicy::MandatoryOn(mandatory_date),
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let bond = ConvertibleBond {
        id: "TEST_MANDATORY_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let _price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(30)).unwrap();
    let _greeks = calculate_convertible_greeks(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(30),
        Some(0.01),
    )
    .unwrap();
}

#[test]
fn test_window_conversion_policy() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let window_start = Date::from_calendar_date(2027, Month::January, 1).unwrap();
    let window_end = Date::from_calendar_date(2029, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(12.0), // 12 shares per bond
        price: None,
        policy: ConversionPolicy::Window {
            start: window_start,
            end: window_end,
        },
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let bond = ConvertibleBond {
        id: "TEST_WINDOW_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Trinomial(40)).unwrap();

    // Price should reflect the delayed conversion option
    let price_val = price;
    assert!(price_val.amount() > 1000.0); // Should have some option value
}

#[test]
fn test_callable_convertible_bond() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let call_date = Date::from_calendar_date(2027, Month::July, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    // Create call schedule
    let mut call_put = CallPutSchedule::default();
    call_put.calls.push(CallPut {
        date: call_date,
        price_pct_of_par: 102.0, // Callable at 102% of par
    });

    let bond = ConvertibleBond {
        id: "TEST_CALLABLE_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: Some(call_put),
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(50)).unwrap();

    // Price should be capped by call option
    let price_val = price;
    // Note: call option may not cap price perfectly due to tree discretization and time value
    assert!(price_val.amount() <= 1700.0); // Should be influenced by call option
}

#[test]
fn test_puttable_convertible_bond() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let put_date = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    // Create put schedule
    let mut call_put = CallPutSchedule::default();
    call_put.puts.push(CallPut {
        date: put_date,
        price_pct_of_par: 98.0, // Puttable at 98% of par
    });

    let bond = ConvertibleBond {
        id: "TEST_PUTTABLE_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: Some(call_put),
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(50)).unwrap();

    // Price should have put option floor
    let price_val = price;
    assert!(price_val.amount() >= 980.0); // Should not fall below put price
}

#[test]
fn test_conversion_price_vs_ratio() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    // Test with conversion price
    let conversion_spec_price = ConversionSpec {
        ratio: None,
        price: Some(100.0), // $100 per share conversion price
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let bond_price = ConvertibleBond {
        id: "TEST_PRICE_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec_price,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    // Test with equivalent conversion ratio
    let conversion_spec_ratio = ConversionSpec {
        ratio: Some(10.0), // 10 shares per bond (equivalent to $100 price for $1000 bond)
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let bond_ratio = ConvertibleBond {
        id: "TEST_RATIO_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec_ratio,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let price1 = price_convertible_bond(
        &bond_price,
        &market_context,
        ConvertibleTreeType::Binomial(30),
    )
    .unwrap();
    let price2 = price_convertible_bond(
        &bond_ratio,
        &market_context,
        ConvertibleTreeType::Binomial(30),
    )
    .unwrap();

    // Should produce nearly identical prices
    let diff_pct = (price1.amount() - price2.amount()).abs() / price1.amount();
    assert!(diff_pct < 0.01); // Within 1%
}

#[test]
fn test_greeks_sanity_checks() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();

    let greeks = calculate_convertible_greeks(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(50),
        Some(0.01),
    )
    .unwrap();

    // Delta should be positive for ITM convertible (increases with stock price)
    assert!(
        greeks.delta > 0.0,
        "Delta should be positive for ITM convertible"
    );
    assert!(
        greeks.delta <= 10.0,
        "Delta should not exceed conversion ratio"
    );

    // Gamma should be non-negative (convexity)
    assert!(greeks.gamma >= 0.0, "Gamma should be non-negative");

    // Vega should be positive (higher vol increases option value)
    assert!(greeks.vega >= 0.0, "Vega should be non-negative");

    // Theta is typically negative for options (time decay)
    // But for convertible bonds it can be positive due to coupon accrual

    // Rho can be positive or negative depending on structure
    // No strong constraint here

    // Price should be reasonable
    assert!(
        greeks.price > 1000.0,
        "Price should exceed face value for ITM convertible"
    );
    assert!(greeks.price < 3000.0, "Price should be reasonable");
}

#[test]
fn test_time_mapping_edge_cases() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::December, 31).unwrap(); // 1 year bond
    let midpoint = Date::from_calendar_date(2025, Month::July, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(5.0),
        price: None,
        policy: ConversionPolicy::Window {
            start: midpoint,
            end: maturity,
        },
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let fixed_coupon = FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.06,
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: BusinessDayConvention::Following,
        calendar_id: None,
        stub: StubKind::None,
    };

    let bond = ConvertibleBond {
        id: "TEST_TIME_MAPPING".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: Some(fixed_coupon),
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    // Test with fewer steps to ensure edge dates are handled
    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(10)).unwrap();

    let price_val = price;
    assert!(
        price_val.amount() > 500.0,
        "Should have reasonable value even with few steps"
    );
}

#[test]
fn test_event_triggered_conversion() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(15.0),
        price: None,
        policy: ConversionPolicy::UponEvent(ConversionEvent::PriceTrigger {
            threshold: 120.0,
            lookback_days: 20,
        }),
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let bond = ConvertibleBond {
        id: "TEST_EVENT_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: None,
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    // Should still price successfully even though event conversion is conservatively disabled
    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(30)).unwrap();

    // Should behave more like a straight bond since conversion is disabled
    let price_val = price;
    assert!(
        price_val.amount() < 2500.0,
        "Should have lower value without conversion option"
    );
}

#[test]
fn test_combined_call_put_convertible() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let call_date = Date::from_calendar_date(2028, Month::January, 1).unwrap();
    let put_date = Date::from_calendar_date(2027, Month::January, 1).unwrap();

    let conversion_spec = ConversionSpec {
        ratio: Some(10.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    // Create combined call/put schedule
    let mut call_put = CallPutSchedule::default();
    call_put.calls.push(CallPut {
        date: call_date,
        price_pct_of_par: 103.0,
    });
    call_put.puts.push(CallPut {
        date: put_date,
        price_pct_of_par: 97.0,
    });

    let bond = ConvertibleBond {
        id: "TEST_CALL_PUT_CONVERTIBLE".to_string(),
        notional: Money::new(1000.0, Currency::USD),
        issue,
        maturity,
        disc_id: "USD-OIS",
        conversion: conversion_spec,
        underlying_equity_id: Some("AAPL".to_string()),
        call_put: Some(call_put),
        fixed_coupon: None,
        floating_coupon: None,
        attributes: Default::default(),
    };

    let market_context = create_test_market_context();

    let price =
        price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(60)).unwrap();

    let price_val = price;
    // Should be bounded by put floor and call ceiling
    assert!(price_val.amount() >= 970.0, "Should respect put floor");
    assert!(
        price_val.amount() <= 1600.0,
        "Should respect call constraints"
    );
}

#[test]
fn test_currency_safety() {
    let bond = create_test_convertible_bond();
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create market context with mismatched currency for equity
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (10.0, 0.741)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market_context = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price(
            "AAPL",
            MarketScalar::Price(Money::new(150.0, Currency::EUR)),
        ) // EUR instead of USD
        .insert_price("AAPL-VOL", MarketScalar::Unitless(0.25))
        .insert_price("AAPL-DIVYIELD", MarketScalar::Unitless(0.02));

    // Should detect currency mismatch and fail
    let price = price_convertible_bond(&bond, &market_context, ConvertibleTreeType::Binomial(20));
    assert!(price.is_err(), "Should fail on currency mismatch");
}
