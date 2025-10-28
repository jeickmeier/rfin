//! Example: Pricing an interest rate cap using Hull-White 1F Monte Carlo.
//!
//! Demonstrates:
//! - Hull-White 1F short rate model
//! - Exact HW1F discretization
//! - Cap payoff with multiple caplets
//! - Currency-safe pricing

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::prelude::*;

fn main() -> finstack_core::Result<()> {
    println!("=== Hull-White 1F Cap Pricing Example ===\n");
    
    // Hull-White parameters (calibrated to market)
    let kappa = 0.03;  // Mean reversion speed
    let sigma = 0.01;  // Short rate volatility (100bp)
    let theta = 0.04;  // Long-term mean (4%)
    
    let hw_params = HullWhite1FParams::new(kappa, sigma, theta);
    let hw_process = HullWhite1FProcess::new(hw_params);
    
    // Exact HW1F discretization
    let disc = ExactHullWhite1F::new();
    
    // Cap specification: quarterly caplets, 2-year maturity
    let strike_rate = 0.05; // 5% cap strike
    let notional = 10_000_000.0; // $10M notional
    
    // Quarterly fixings
    let fixing_dates = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
    let accrual_fractions = vec![0.25; 8]; // All quarterly
    
    // Simple discount factors (flat 4% curve)
    let discount_factors: Vec<f64> = fixing_dates
        .iter()
        .map(|&t| (-0.04 * t).exp())
        .collect();
    
    let cap_payoff = CapPayoff::new(
        strike_rate,
        notional,
        fixing_dates.clone(),
        accrual_fractions,
        discount_factors,
        Currency::USD,
    );
    
    // MC engine configuration
    let config = McEngineConfig::new(100_000, 42, TimeGrid::uniform(0.0, 2.0, 500))
        .with_parallel(true)
        .with_chunk_size(1000);
    
    let engine = McEngine::new(config);
    
    // Initial short rate
    let initial_state = vec![0.04]; // Start at 4%
    
    // Price the cap
    println!("Pricing cap with:");
    println!("  Strike: {:.2}%", strike_rate * 100.0);
    println!("  Notional: ${:,.0}", notional);
    println!("  Tenor: 2 years, quarterly");
    println!("  Paths: 100,000");
    println!("  HW1F params: κ={}, σ={}, θ={}\n", kappa, sigma, theta);
    
    let rng = PhiloxRng::new(42);
    
    let result = engine.price(
        &rng,
        &hw_process,
        &disc,
        &initial_state,
        &cap_payoff,
        Currency::USD,
        1.0, // Discount already in payoff
    )?;
    
    println!("Results:");
    println!("  Cap PV: ${:,.2} ± ${:,.2}", result.mean, result.stderr * 1.96);
    println!("  95% CI: [${:,.2}, ${:,.2}]", result.ci_95.0, result.ci_95.1);
    println!("  Std Error: ${:,.2}", result.stderr);
    println!("  Paths Used: {}", result.count);
    
    // Caplet approximation (rough check)
    let num_caplets = fixing_dates.len();
    let avg_caplet = result.mean / num_caplets as f64;
    println!("\n  Average caplet value: ${:,.2}", avg_caplet);
    
    Ok(())
}

