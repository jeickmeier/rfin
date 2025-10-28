//! Example: Basket options and exchange options.
//!
//! Demonstrates:
//! - Multi-asset basket payoffs
//! - Margrabe formula for exchange options
//! - Correlation effects on basket value

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::payoff::basket::*;

fn main() -> finstack_core::Result<()> {
    println!("=== Basket Options Example ===\n");
    
    // Two-asset exchange option (Margrabe formula)
    let s1 = 105.0;
    let s2 = 100.0;
    let sigma1 = 0.25;
    let sigma2 = 0.20;
    let time_to_maturity = 1.0;
    let q1 = 0.02;
    let q2 = 0.03;
    
    println!("Exchange Option (option to exchange asset 2 for asset 1):");
    println!("  Asset 1: ${:.2}, vol={:.1}%, div={:.1}%", s1, sigma1 * 100.0, q1 * 100.0);
    println!("  Asset 2: ${:.2}, vol={:.1}%, div={:.1}%\n", s2, sigma2 * 100.0, q2 * 100.0);
    
    // Test different correlations
    for rho in [0.0, 0.3, 0.6, 0.9] {
        let price = margrabe_exchange_option(
            s1, s2,
            sigma1, sigma2,
            rho,
            time_to_maturity,
            q1, q2,
        );
        
        println!("  ρ = {:.1}: Exchange option value = ${:.4}", rho, price);
    }
    
    println!("\nInterpretation:");
    println!("  - Higher correlation → lower option value");
    println!("  - As ρ → 1, both assets move together, reducing optionality");
    println!("  - As ρ → 0, assets diversify, increasing exchange value");
    
    // Basket types
    println!("\n=== Basket Aggregations ===\n");
    
    let values = vec![95.0, 100.0, 105.0];
    println!("Asset values: ${:.0}, ${:.0}, ${:.0}", values[0], values[1], values[2]);
    
    let basket_sum = BasketCall::new(100.0, 1.0, BasketType::Sum, 3, 1, Currency::USD);
    println!("  Sum basket: ${:.2}", basket_sum.compute_basket_value(&values));
    
    let basket_avg = BasketCall::new(100.0, 1.0, BasketType::Average, 3, 1, Currency::USD);
    println!("  Average basket: ${:.2}", basket_avg.compute_basket_value(&values));
    
    let basket_max = BasketCall::new(100.0, 1.0, BasketType::Max, 3, 1, Currency::USD);
    println!("  Max (best-of): ${:.2}", basket_max.compute_basket_value(&values));
    
    let basket_min = BasketCall::new(100.0, 1.0, BasketType::Min, 3, 1, Currency::USD);
    println!("  Min (worst-of): ${:.2}", basket_min.compute_basket_value(&values));
    
    println!("\nBasket inequality: Max ≥ Average ≥ Min");
    
    Ok(())
}

