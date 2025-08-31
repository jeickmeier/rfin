//! Comprehensive tests for convertible bond pricing framework.

use super::model::{price_convertible_bond, calculate_convertible_greeks, calculate_parity, ConvertibleTreeType};
use super::{ConvertibleBond, ConversionPolicy, ConversionSpec, AntiDilutionPolicy, DividendAdjustment};

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency, BusinessDayConvention, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::primitives::MarketScalar;
use finstack_core::money::Money;

use crate::cashflow::builder::types::{FixedCouponSpec, CouponType};
use crate::instruments::options::models::{BinomialTree, TrinomialTree, TreeModel, NodeState, TreeValuator, single_factor_equity_state};

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
        .knots([(0.0, 1.0), (10.0, 0.741)] ) // ~3% rate: e^(-0.03*10) = 0.741
        .linear_df()
        .build()
        .unwrap();

    MarketContext::new()
        .with_discount(discount_curve)
        .with_price("AAPL", MarketScalar::Unitless(150.0)) // $150 stock price
        .with_price("AAPL-VOL", MarketScalar::Unitless(0.25)) // 25% volatility
        .with_price("AAPL-DIVYIELD", MarketScalar::Unitless(0.02)) // 2% dividend yield
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
    
    let price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(50),
    );
    
    assert!(price.is_ok());
    let price = price.unwrap();
    
    // Should be worth at least the conversion value
    let conversion_value = 150.0 * 10.0; // $1,500
    assert!(price.amount() >= conversion_value);
    
    // Should be in a reasonable range for this scenario
    assert!(price.amount() > 1400.0 && price.amount() < 2000.0);
    assert_eq!(price.currency(), Currency::USD);
}

#[test]
fn test_convertible_bond_pricing_trinomial() {
    let bond = create_test_convertible_bond();
    let market_context = create_test_market_context();
    
    let price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Trinomial(50),
    );
    
    assert!(price.is_ok());
    let price = price.unwrap();
    
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
    
    let bin_price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(100),
    ).unwrap();
    
    let tri_price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Trinomial(100),
    ).unwrap();
    
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
    market_context = market_context.with_price("AAPL", MarketScalar::Unitless(50.0));
    
    let price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(50),
    ).unwrap();
    
    // Should be worth close to bond value (conversion value = 50*10 = 500)
    // Bond should be worth close to its debt value
    assert!(price.amount() < 1200.0); // Less than deep ITM case
    assert!(price.amount() > 800.0);  // But more than just conversion value
}

#[test]
fn test_low_volatility_convertible() {
    let bond = create_test_convertible_bond();
    let mut market_context = create_test_market_context();
    
    // Set low volatility (but not too low to avoid numerical issues)
    market_context = market_context.with_price("AAPL-VOL", MarketScalar::Unitless(0.05));
    
    let price = price_convertible_bond(
        &bond,
        &market_context,
        ConvertibleTreeType::Binomial(20),
    );
    
    // Should work with low volatility
    assert!(price.is_ok());
    let price = price.unwrap();
    
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
        
        fn value_at_node(&self, _state: &NodeState, continuation_value: finstack_core::F) -> finstack_core::Result<finstack_core::F> {
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
