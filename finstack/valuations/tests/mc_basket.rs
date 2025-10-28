//! Integration tests for basket option pricing.
//!
//! Validates basket payoffs (sum, average, max, min) and exchange options
//! against analytical benchmarks (Margrabe formula).

use finstack_core::currency::Currency;
use finstack_core::Result;
use finstack_valuations::instruments::common::mc::payoff::basket::*;
use finstack_valuations::instruments::common::mc::traits::Payoff;

#[test]
fn test_basket_type_aggregations() -> Result<()> {
    // Test basket aggregation logic
    let values = vec![30.0, 40.0, 50.0];
    
    // Sum
    let basket_sum = BasketCall::new(100.0, 1.0, BasketType::Sum, 3, 1, Currency::USD);
    assert_eq!(basket_sum.compute_basket_value(&values), 120.0);
    
    // Average
    let basket_avg = BasketCall::new(100.0, 1.0, BasketType::Average, 3, 1, Currency::USD);
    assert_eq!(basket_avg.compute_basket_value(&values), 40.0);
    
    // Max
    let basket_max = BasketCall::new(100.0, 1.0, BasketType::Max, 3, 1, Currency::USD);
    assert_eq!(basket_max.compute_basket_value(&values), 50.0);
    
    // Min
    let basket_min = BasketCall::new(100.0, 1.0, BasketType::Min, 3, 1, Currency::USD);
    assert_eq!(basket_min.compute_basket_value(&values), 30.0);
    
    Ok(())
}


#[test]
fn test_exchange_option_zero_correlation() -> Result<()> {
    // Test exchange option with zero correlation
    let s1 = 100.0;
    let s2 = 100.0;
    let sigma1 = 0.2;
    let sigma2 = 0.2;
    let rho = 0.0;
    let time_to_maturity = 0.5;
    
    let analytical = margrabe_exchange_option(
        s1, s2,
        sigma1, sigma2,
        rho,
        time_to_maturity,
        0.0, 0.0,
    );
    
    // ATM with zero correlation should have positive time value
    assert!(analytical > 0.0);
    
    println!("Exchange (rho=0, ATM): {:.6}", analytical);
    
    Ok(())
}

#[test]
fn test_exchange_option_perfect_correlation() -> Result<()> {
    // Test exchange option with perfect correlation
    let s1 = 100.0;
    let s2 = 100.0;
    let sigma1 = 0.2;
    let sigma2 = 0.2;
    let rho = 1.0;
    let time_to_maturity = 1.0;
    
    let analytical = margrabe_exchange_option(
        s1, s2,
        sigma1, sigma2,
        rho,
        time_to_maturity,
        0.0, 0.0,
    );
    
    // Perfect correlation, same vol, same div, ATM
    // Combined vol = sqrt(σ1² + σ2² - 2ρσ1σ2) = sqrt(0.04 + 0.04 - 0.08) = 0
    // So option value should be intrinsic = max(S1-S2, 0) = 0
    assert!(analytical.abs() < 1e-6, "Expected ~0, got {}", analytical);
    
    println!("Exchange (rho=1, ATM, same params): {:.6}", analytical);
    
    Ok(())
}

#[test]
fn test_margrabe_intrinsic_value() -> Result<()> {
    // Deep in the money: S1 >> S2
    let s1 = 150.0;
    let s2 = 100.0;
    let sigma1 = 0.1;
    let sigma2 = 0.1;
    let rho = 0.5;
    let time_to_maturity = 0.01; // Near expiry
    
    let analytical = margrabe_exchange_option(
        s1, s2,
        sigma1, sigma2,
        rho,
        time_to_maturity,
        0.0, 0.0,
    );
    
    // Should be close to intrinsic value (50) for near-expiry deep ITM
    let intrinsic = s1 - s2;
    assert!(analytical >= intrinsic * 0.95); // At least 95% of intrinsic
    assert!(analytical <= intrinsic * 1.05); // Not much more than intrinsic
    
    println!("Exchange (deep ITM, near expiry): {:.6}", analytical);
    println!("Intrinsic value: {:.6}", intrinsic);
    
    Ok(())
}

#[test]
fn test_basket_call_intrinsic() -> Result<()> {
    // Test basket call payoff calculation
    let mut basket = BasketCall::new(100.0, 1000.0, BasketType::Sum, 2, 0, Currency::USD);
    
    // Test ITM: sum = 120 > strike 100
    let values = vec![60.0, 60.0];
    let basket_value = basket.compute_basket_value(&values);
    assert_eq!(basket_value, 120.0);
    
    // Manually set terminal value and check payoff
    basket.terminal_basket_value = 120.0;
    let payoff = basket.value(Currency::USD);
    assert_eq!(payoff.amount(), 20.0 * 1000.0); // (120 - 100) * 1000
    
    // Test OTM: sum = 80 < strike 100
    basket.terminal_basket_value = 80.0;
    let payoff_otm = basket.value(Currency::USD);
    assert_eq!(payoff_otm.amount(), 0.0); // max(80-100, 0) = 0
    
    Ok(())
}

#[test]
fn test_basket_put_intrinsic() -> Result<()> {
    // Test basket put payoff calculation
    let mut basket = BasketPut::new(100.0, 500.0, BasketType::Average, 3, 0, Currency::USD);
    
    // Test ITM: avg = 90 < strike 100
    let values = vec![80.0, 90.0, 100.0];
    let basket_value = basket.compute_basket_value(&values);
    assert_eq!(basket_value, 90.0);
    
    basket.terminal_basket_value = 90.0;
    let payoff = basket.value(Currency::USD);
    assert_eq!(payoff.amount(), 10.0 * 500.0); // (100 - 90) * 500
    
    // Test OTM: avg = 110 > strike 100
    basket.terminal_basket_value = 110.0;
    let payoff_otm = basket.value(Currency::USD);
    assert_eq!(payoff_otm.amount(), 0.0); // max(100-110, 0) = 0
    
    Ok(())
}

