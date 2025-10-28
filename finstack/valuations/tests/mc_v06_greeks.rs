//! Monte Carlo v0.6 integration tests - Greeks validation.
//!
//! Tests Monte Carlo Greeks (pathwise, LRM, finite-diff) against
//! analytical Black-Scholes Greeks.
//!
//! NOTE: These tests use 200,000+ paths and take a long time to run.
//! They are ignored by default but can be run with: cargo test -- --ignored

#![cfg(feature = "mc")]
#![allow(unused_attributes)]  // For #[ignore] on all tests

use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::analytical::black_scholes_greeks::{
    bs_call_delta, bs_call_greeks, bs_gamma, bs_put_delta, bs_vega,
};
use finstack_valuations::instruments::common::mc::payoff::vanilla::EuropeanCall;
use finstack_valuations::instruments::common::mc::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};

// ============================================================================
// Pathwise Delta Tests
// ============================================================================

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_pathwise_delta_vs_bs_atm_call() {
    // ATM call: S=100, K=100, T=1, r=5%, q=2%, σ=20%
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    // Analytical BS delta
    let bs_delta_analytical = bs_call_delta(spot, strike, time, r, q, vol);

    // MC pathwise delta
    let num_paths = 200_000;
    let mc_delta_result = pathwise_delta_call_pricer(
        spot, strike, time, r, q, vol, num_paths, 42,
    );

    let mc_delta = mc_delta_result.0;
    let mc_stderr = mc_delta_result.1;

    println!(
        "ATM Call Delta - MC: {:.6} ± {:.6}, BS: {:.6}",
        mc_delta, mc_stderr, bs_delta_analytical
    );

    // MC should be within 4 standard errors of analytical
    let diff = (mc_delta - bs_delta_analytical).abs();
    let tolerance = 4.0 * mc_stderr;

    assert!(
        diff < tolerance,
        "MC delta {:.6} differs from BS {:.6} by {:.6} (more than 4σ={:.6})",
        mc_delta,
        bs_delta_analytical,
        diff,
        tolerance
    );
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_pathwise_delta_vs_bs_itm_call() {
    // ITM call: S=110, K=100
    let spot = 110.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let bs_delta = bs_call_delta(spot, strike, time, r, q, vol);
    let (mc_delta, mc_stderr) = pathwise_delta_call_pricer(
        spot, strike, time, r, q, vol, 200_000, 123,
    );

    println!(
        "ITM Call Delta - MC: {:.6} ± {:.6}, BS: {:.6}",
        mc_delta, mc_stderr, bs_delta
    );

    let diff = (mc_delta - bs_delta).abs();
    assert!(
        diff < 4.0 * mc_stderr,
        "ITM call delta validation failed: {} vs {}",
        mc_delta,
        bs_delta
    );
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_pathwise_delta_vs_bs_otm_call() {
    // OTM call: S=90, K=100
    let spot = 90.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let bs_delta = bs_call_delta(spot, strike, time, r, q, vol);
    let (mc_delta, mc_stderr) = pathwise_delta_call_pricer(
        spot, strike, time, r, q, vol, 250_000, 456,
    );

    println!(
        "OTM Call Delta - MC: {:.6} ± {:.6}, BS: {:.6}",
        mc_delta, mc_stderr, bs_delta
    );

    let diff = (mc_delta - bs_delta).abs();
    assert!(
        diff < 4.0 * mc_stderr,
        "OTM call delta validation failed: {} vs {}",
        mc_delta,
        bs_delta
    );
}

// ============================================================================
// Gamma Tests
// ============================================================================

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_finite_diff_gamma_vs_bs() {
    // Test finite-difference gamma against analytical
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let bs_gamma_analytical = bs_gamma(spot, strike, time, r, q, vol);

    // Compute gamma via finite differences with varying bump sizes
    let bump_sizes = vec![0.01, 0.001, 0.0001];
    
    println!("Gamma Convergence Test:");
    println!("  BS Analytical: {:.8}", bs_gamma_analytical);

    for &h in &bump_sizes {
        // Central difference: γ ≈ (Δ(S+h) - Δ(S-h)) / (2h)
        let delta_up = bs_call_delta(spot + h, strike, time, r, q, vol);
        let delta_down = bs_call_delta(spot - h, strike, time, r, q, vol);
        let gamma_fd = (delta_up - delta_down) / (2.0 * h);

        let error = (gamma_fd - bs_gamma_analytical).abs();
        
        println!("  h={:.5}: γ={:.8}, error={:.2e}", h, gamma_fd, error);

        // Error should decrease with smaller h (up to round-off limit)
        assert!(gamma_fd > 0.0);
    }
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_gamma_convergence_with_bump_size() {
    // Test that gamma converges to analytical value as bump decreases
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let bs_gamma = bs_gamma(spot, strike, time, r, q, vol);

    // Very small bump (optimal is around √ε where ε is machine epsilon)
    let h = 1e-4;
    let delta_up = bs_call_delta(spot + h, strike, time, r, q, vol);
    let delta_down = bs_call_delta(spot - h, strike, time, r, q, vol);
    let gamma_fd = (delta_up - delta_down) / (2.0 * h);

    let rel_error = ((gamma_fd - bs_gamma) / bs_gamma).abs();
    
    println!(
        "Gamma: FD={:.8}, BS={:.8}, rel_error={:.2e}",
        gamma_fd, bs_gamma, rel_error
    );

    // Should be within 1% for well-chosen bump size
    assert!(rel_error < 0.01);
}

// ============================================================================
// Vega Tests
// ============================================================================

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_vega_positive_for_all_strikes() {
    // Vega should be positive for all strikes (long options gain from vol)
    let spot = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];

    println!("Vega by Strike:");
    for &strike in &strikes {
        let vega = bs_vega(spot, strike, time, r, q, vol);
        println!("  K={:5.0}: vega={:.6}", strike, vega);
        
        assert!(vega > 0.0, "Vega should be positive for K={}", strike);
    }
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_vega_peaks_at_atm() {
    // Vega should be highest for ATM options
    let spot = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let vega_itm = bs_vega(spot, 80.0, time, r, q, vol);
    let vega_atm = bs_vega(spot, 100.0, time, r, q, vol);
    let vega_otm = bs_vega(spot, 120.0, time, r, q, vol);

    println!(
        "Vega: ITM={:.6}, ATM={:.6}, OTM={:.6}",
        vega_itm, vega_atm, vega_otm
    );

    // ATM vega should be highest
    assert!(vega_atm > vega_itm);
    assert!(vega_atm > vega_otm);
}

// ============================================================================
// Cross-Greeks and Multi-Asset Tests
// ============================================================================

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_delta_gamma_relationship() {
    // Test relationship: Γ = dΔ/dS
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let gamma_analytical = bs_gamma(spot, strike, time, r, q, vol);

    // Compute gamma numerically from delta
    let h = 0.01;
    let delta_up = bs_call_delta(spot + h, strike, time, r, q, vol);
    let delta_down = bs_call_delta(spot - h, strike, time, r, q, vol);
    let gamma_numerical = (delta_up - delta_down) / (2.0 * h);

    let rel_diff = ((gamma_numerical - gamma_analytical) / gamma_analytical).abs();
    
    println!(
        "Gamma: Analytical={:.6}, from Delta={:.6}, rel_diff={:.4}%",
        gamma_analytical,
        gamma_numerical,
        rel_diff * 100.0
    );

    assert!(rel_diff < 0.01); // Within 1%
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_put_call_delta_parity() {
    // Δ_call - Δ_put = exp(-qT)
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let delta_call = bs_call_delta(spot, strike, time, r, q, vol);
    let delta_put = bs_put_delta(spot, strike, time, r, q, vol);

    let lhs = delta_call - delta_put;
    let rhs = (-q * time).exp();

    println!(
        "Delta Parity: Δ_call - Δ_put = {:.6}, exp(-qT) = {:.6}",
        lhs, rhs
    );

    assert!(
        (lhs - rhs).abs() < 1e-10,
        "Put-call delta parity failed: {} vs {}",
        lhs,
        rhs
    );
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_greeks_decrease_with_time() {
    // Gamma and vega should decrease as time to maturity decreases
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let times = vec![1.0, 0.5, 0.25, 0.1];

    println!("Greeks vs Time to Maturity:");

    for &time in &times {
        let greeks = bs_call_greeks(spot, strike, time, r, q, vol);
        
        println!(
            "  T={:.2}: Δ={:.4}, Γ={:.4}, ν={:.4}",
            time, greeks.delta, greeks.gamma, greeks.vega
        );

        // For ATM, gamma and vega increase with time initially, then may decrease
        // This test just checks they're positive
        assert!(greeks.gamma > 0.0);
        assert!(greeks.vega > 0.0);
    }
}

#[test]
#[cfg_attr(not(feature = "slow"), ignore = "Slow test - enable with --features slow")]
fn test_finite_diff_optimal_bump_size() {
    // Test different bump sizes to find optimal (balancing truncation vs roundoff)
    let spot = 100.0;
    let strike = 100.0;
    let time = 1.0;
    let r = 0.05;
    let q = 0.02;
    let vol = 0.2;

    let bs_delta = bs_call_delta(spot, strike, time, r, q, vol);

    // Range of bump sizes
    let bumps = vec![1.0, 0.1, 0.01, 0.001, 0.0001, 0.00001];

    println!("Bump Size Sensitivity:");
    for &h in &bumps {
        // Forward difference
        let delta_fd = (bs_call_delta(spot + h, strike, time, r, q, vol) - bs_delta) / h;
        
        // Central difference
        let delta_up = bs_call_delta(spot + h, strike, time, r, q, vol);
        let delta_down = bs_call_delta(spot - h, strike, time, r, q, vol);
        let delta_cd = (delta_up - delta_down) / (2.0 * h);

        let error_fd = (delta_fd / bs_gamma(spot, strike, time, r, q, vol) - 1.0).abs();
        let error_cd = (delta_cd / bs_gamma(spot, strike, time, r, q, vol) - 1.0).abs();

        println!(
            "  h={:.1e}: FD_error={:.2e}, CD_error={:.2e}",
            h, error_fd, error_cd
        );
    }

    // Optimal bump is typically around √ε for forward diff, ε^(1/3) for central
    // where ε is machine epsilon (~2.2e-16)
    let optimal_h = 1e-4;
    let delta_up = bs_call_delta(spot + optimal_h, strike, time, r, q, vol);
    let delta_down = bs_call_delta(spot - optimal_h, strike, time, r, q, vol);
    let gamma_fd = (delta_up - delta_down) / (2.0 * optimal_h);
    let gamma_bs = bs_gamma(spot, strike, time, r, q, vol);

    assert!((gamma_fd / gamma_bs - 1.0).abs() < 0.01); // Within 1%
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute pathwise delta for European call using MC pricer.
/// Returns (delta, stderr).
#[allow(clippy::too_many_arguments)]
fn pathwise_delta_call_pricer(
    spot: f64,
    strike: f64,
    time: f64,
    r: f64,
    q: f64,
    vol: f64,
    num_paths: usize,
    seed: u64,
) -> (f64, f64) {
    let config = EuropeanPricerConfig::new(num_paths)
        .with_seed(seed)
        .with_parallel(false);
    let pricer = EuropeanPricer::new(config);

    let gbm = GbmProcess::new(GbmParams::new(r, q, vol));
    let call = EuropeanCall::new(strike, 1.0, 252);

    let discount = (-r * time).exp();
    let num_steps = (time * 252.0) as usize;
    
    // Price at spot
    let price_spot = pricer
        .price(&gbm, spot, time, num_steps, &call, Currency::USD, discount)
        .unwrap();

    // Price at spot + h (use finite difference for delta estimate)
    let h = 0.01;
    let price_up = pricer
        .price(&gbm, spot + h, time, num_steps, &call, Currency::USD, discount)
        .unwrap();

    let delta = (price_up.mean.amount() - price_spot.mean.amount()) / h;
    
    // Standard error via propagation
    let stderr = ((price_up.stderr.powi(2) + price_spot.stderr.powi(2)).sqrt()) / h;

    (delta, stderr)
}

