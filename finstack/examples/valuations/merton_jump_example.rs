//! Example: Pricing options under Merton jump-diffusion.
//!
//! Demonstrates:
//! - Merton jump-diffusion model
//! - Jump-Euler discretization
//! - Comparison with Black-Scholes (no jumps)

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::prelude::*;

fn main() -> finstack_core::Result<()> {
    println!("=== Merton Jump-Diffusion Example ===\n");
    
    // Market parameters
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let time_to_maturity = 1.0;
    
    // Diffusion volatility
    let sigma = 0.20;
    
    // Jump parameters
    let lambda = 1.0;   // 1 jump per year on average
    let mu_j = -0.05;   // Slightly negative jumps (crashes)
    let sigma_j = 0.10; // Jump volatility
    
    let jump_params = MertonJumpParams::new(r, q, sigma, lambda, mu_j, sigma_j);
    let merton_process = MertonJumpProcess::new(jump_params);
    
    // Jump-Euler discretization
    let disc = JumpEuler::new();
    
    // European call payoff
    let call_payoff = EuropeanCall::new(strike, 1.0, 252);
    
    // MC engine
    let config = McEngineConfig::new(100_000, 42, TimeGrid::uniform(0.0, time_to_maturity, 252))
        .with_parallel(true);
    let engine = McEngine::new(config);
    
    let rng = PhiloxRng::new(42);
    let initial_state = vec![spot];
    
    println!("Pricing European call with jump-diffusion:");
    println!("  Spot: ${:.2}", spot);
    println!("  Strike: ${:.2}", strike);
    println!("  Volatility: {:.1}%", sigma * 100.0);
    println!("  Jump intensity: {:.1} jumps/year", lambda);
    println!("  Jump mean: {:.2}%", mu_j * 100.0);
    println!("  Jump vol: {:.1}%\n", sigma_j * 100.0);
    
    let discount_factor = (-r * time_to_maturity).exp();
    
    let result = engine.price(
        &rng,
        &merton_process,
        &disc,
        &initial_state,
        &call_payoff,
        Currency::USD,
        discount_factor,
    )?;
    
    println!("Merton Jump-Diffusion:");
    println!("  Price: ${:.4} ± ${:.4}", result.mean, result.stderr * 1.96);
    println!("  95% CI: [${:.4}, ${:.4}]", result.ci_95.0, result.ci_95.1);
    
    // For comparison, price with Black-Scholes (no jumps)
    let bs_price = black_scholes_call(
        spot,
        strike,
        r,
        q,
        sigma,
        time_to_maturity,
    );
    
    println!("\nBlack-Scholes (no jumps):");
    println!("  Price: ${:.4}", bs_price);
    
    println!("\nDifference: ${:.4} ({:.1}%)", 
        result.mean - bs_price,
        (result.mean - bs_price) / bs_price * 100.0
    );
    
    println!("\nInterpretation:");
    if result.mean < bs_price {
        println!("  Jump-diffusion price is LOWER due to negative jump compensation");
    } else {
        println!("  Jump-diffusion price is HIGHER - jumps add value");
    }
    
    Ok(())
}

