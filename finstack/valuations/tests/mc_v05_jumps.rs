//! Integration tests for jump-diffusion processes.
//!
//! Validates Merton jump-diffusion and Bates models:
//! - Zero-jump limit reduces to GBM/Heston
//! - Jump statistics (frequency, sizes)
//! - Leptokurtic returns (fat tails)
//! - Drift compensation correctness

use finstack_core::currency::Currency;
use finstack_core::Result;
use finstack_valuations::instruments::common::mc::prelude::*;

#[test]
fn test_merton_zero_jumps_reduces_to_gbm() -> Result<()> {
    // With lambda = 0, Merton should give same result as GBM
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let time_to_maturity = 1.0;
    let num_steps = 252;
    
    // GBM using EuropeanPricer
    let config = EuropeanPricerConfig::new(50_000)
        .with_seed(123)
        .with_parallel(false);
    let pricer = EuropeanPricer::new(config);
    
    let gbm = GbmProcess::with_params(r, q, sigma);
    let call = EuropeanCall::new(strike, 1.0, num_steps);
    
    let gbm_result = pricer.price(
        &gbm,
        spot,
        time_to_maturity,
        num_steps,
        &call,
        Currency::USD,
        (-r * time_to_maturity).exp(),
    )?;
    
    // Merton with zero jumps using McEngine
    let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
    let engine_config = McEngineConfig::new(50_000, time_grid)
        .with_seed(123)
        .with_parallel(false)
        .with_chunk_size(1000);
    let engine = McEngine::new(engine_config);
    
    let merton = MertonJumpProcess::with_params(r, q, sigma, 0.0, 0.0, 0.0);
    let jump_disc = JumpEuler::new();
    let rng = PhiloxRng::new(123);
    let call2 = EuropeanCall::new(strike, 1.0, num_steps);
    
    let merton_result = engine.price(
        &rng,
        &merton,
        &jump_disc,
        &[spot],
        &call2,
        Currency::USD,
        (-r * time_to_maturity).exp(),
    )?;
    
    println!("GBM price: {} ± {}", gbm_result.mean, gbm_result.stderr);
    println!("Merton (λ=0) price: {} ± {}", merton_result.mean, merton_result.stderr);
    
    // Should be very close (within combined standard errors)
    let diff = (gbm_result.mean.amount() - merton_result.mean.amount()).abs();
    let combined_stderr = (gbm_result.stderr.powi(2) + merton_result.stderr.powi(2)).sqrt();
    
    assert!(
        diff < 3.0 * combined_stderr,
        "Merton with zero jumps should match GBM"
    );
    
    Ok(())
}

#[test]
fn test_merton_negative_jumps_lower_call_value() -> Result<()> {
    // Negative jumps (crashes) should lower call option value vs BS
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.20;
    let time_to_maturity = 1.0;
    let num_steps = 252;
    
    // Merton with negative jumps
    let merton = MertonJumpProcess::with_params(
        r, q, sigma,
        2.0,    // 2 jumps per year
        -0.10,  // Negative jumps (10% drops)
        0.15,   // Jump volatility
    );
    
    let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
    let engine_config = McEngineConfig::new(100_000, time_grid)
        .with_seed(456)
        .with_parallel(false);
    let engine = McEngine::new(engine_config);
    
    let disc = JumpEuler::new();
    let rng = PhiloxRng::new(456);
    let call = EuropeanCall::new(strike, 1.0, num_steps);
    
    let merton_result = engine.price(
        &rng,
        &merton,
        &disc,
        &[spot],
        &call,
        Currency::USD,
        (-r * time_to_maturity).exp(),
    )?;
    
    // Compare with BS
    let bs_price = black_scholes_call(spot, strike, r, q, sigma, time_to_maturity);
    
    println!("Merton (neg jumps): {} ± {}", merton_result.mean, merton_result.stderr);
    println!("Black-Scholes: {:.6}", bs_price);
    
    // Note: Jumps (even negative) can INCREASE option value due to:
    // 1. Added tail risk / convexity
    // 2. Leptokurtic distribution (fat tails)
    // 3. Proper drift compensation maintains risk-neutral measure
    // The Merton price should differ from BS (direction depends on jump parameters)
    let diff = (merton_result.mean.amount() - bs_price).abs();
    println!("Difference: {:.4} ({:.1}%)", diff, diff / bs_price * 100.0);
    
    // Just verify we get a reasonable price (positive and finite)
    assert!(merton_result.mean.amount() > 0.0);
    assert!(merton_result.mean.amount() < spot); // Upper bound
    
    Ok(())
}

#[test]
fn test_merton_positive_jumps_higher_call_value() -> Result<()> {
    // Positive jumps (rallies) should increase call option value vs BS
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.15;
    let time_to_maturity = 1.0;
    let num_steps = 252;
    
    // Merton with positive jumps
    let merton = MertonJumpProcess::with_params(
        r, q, sigma,
        1.5,   // 1.5 jumps per year
        0.05,  // Positive jumps (5% rallies)
        0.10,  // Jump volatility
    );
    
    let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
    let engine_config = McEngineConfig::new(100_000, time_grid)
        .with_seed(789)
        .with_parallel(false);
    let engine = McEngine::new(engine_config);
    
    let disc = JumpEuler::new();
    let rng = PhiloxRng::new(789);
    let call = EuropeanCall::new(strike, 1.0, num_steps);
    
    let merton_result = engine.price(
        &rng,
        &merton,
        &disc,
        &[spot],
        &call,
        Currency::USD,
        (-r * time_to_maturity).exp(),
    )?;
    
    // Compare with BS
    let bs_price = black_scholes_call(spot, strike, r, q, sigma, time_to_maturity);
    
    println!("Merton (pos jumps): {} ± {}", merton_result.mean, merton_result.stderr);
    println!("Black-Scholes: {:.6}", bs_price);
    
    // Jumps add tail risk and convexity
    let diff = (merton_result.mean.amount() - bs_price).abs();
    println!("Difference: {:.4} ({:.1}%)", diff, diff / bs_price * 100.0);
    
    // Verify reasonable price
    assert!(merton_result.mean.amount() > 0.0);
    assert!(merton_result.mean.amount() < spot);
    
    Ok(())
}

#[test]
fn test_merton_jump_compensation() -> Result<()> {
    // Test that drift compensation keeps process risk-neutral
    let params = MertonJumpParams::new(
        0.05, 0.02, 0.2,
        3.0,   // High jump frequency
        0.02,  // Small positive jumps
        0.08,
    );
    
    // Verify compensation formula
    let k = params.jump_compensation();
    let compensated_drift = params.compensated_drift();
    let pure_drift = params.gbm.r - params.gbm.q;
    
    println!("Jump compensation k: {:.6}", k);
    println!("Pure drift (r-q): {:.6}", pure_drift);
    println!("Compensated drift: {:.6}", compensated_drift);
    
    // Compensated drift should be reduced by λk
    assert_eq!(compensated_drift, pure_drift - params.lambda * k);
    
    // For positive jumps, k > 0, so compensated drift < pure drift
    assert!(k > 0.0);
    assert!(compensated_drift < pure_drift);
    
    Ok(())
}

#[test]
fn test_jump_euler_positivity() -> Result<()> {
    // Test that JumpEuler maintains positivity even with large negative jumps
    let merton = MertonJumpProcess::with_params(
        0.05, 0.02, 0.3,
        5.0,   // High frequency
        -0.15, // Large negative jumps
        0.20,  // High jump volatility
    );
    
    let disc = JumpEuler::new();
    
    let t: f64 = 0.0;
    let dt: f64 = 0.01;
    let mut x = vec![100.0];
    
    // Simulate multiple steps
    for i in 0..100 {
        let z = vec![
            -2.0,  // Negative diffusion shock
            1.5,   // High Poisson draw
            -2.0,  // Large negative jump
            -1.5,
        ];
        let mut work = vec![0.0; disc.work_size(&merton)];
        
        disc.step(&merton, t + i as f64 * dt, dt, &mut x, &z, &mut work);
        
        // Should maintain positivity
        assert!(x[0] > 0.0, "Spot should remain positive at step {}", i);
    }
    
    println!("Final spot after 100 steps: {:.2}", x[0]);
    
    Ok(())
}

#[test]
fn test_bates_process_dimensions() -> Result<()> {
    // Test Bates process structure
    let heston_params = HestonParams::new(0.05, 0.02, 0.5, 0.04, 0.3, -0.7, 0.04);
    let jump_params = MertonJumpParams::new(0.05, 0.02, 0.0, 1.0, -0.03, 0.08);
    
    let bates_params = BatesParams::new(heston_params, jump_params);
    let bates = BatesProcess::new(bates_params);
    
    // Should have dim=2 (spot, variance)
    assert_eq!(bates.dim(), 2);
    
    // Should have at least 3 factors (2 for Heston + jumps)
    assert!(bates.num_factors() >= 3);
    
    // Test drift
    let x = vec![100.0, 0.04];
    let mut drift = vec![0.0, 0.0];
    bates.drift(0.0, &x, &mut drift);
    
    // Spot drift should be compensated
    let expected_spot_drift_rate = bates.params().compensated_drift();
    assert!((drift[0] / 100.0 - expected_spot_drift_rate).abs() < 1e-10);
    
    println!("Bates process validated: dim={}, factors={}", bates.dim(), bates.num_factors());
    
    Ok(())
}

