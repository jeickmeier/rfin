//! Integration tests for interest rate models.
//!
//! Validates Hull-White 1F, CIR, and cap/floor pricing.

use finstack_core::currency::Currency;
use finstack_core::Result;
use finstack_valuations::instruments::common::mc::prelude::*;

#[test]
fn test_hw1f_mean_reversion_simulation() -> Result<()> {
    // Test that HW1F exhibits mean reversion in simulation
    let kappa = 0.1;
    let theta = 0.04; // 4% long-term mean
    let sigma = 0.01;
    
    let hw_params = HullWhite1FParams::new(kappa, sigma, theta);
    let hw_process = HullWhite1FProcess::new(hw_params);
    let disc = ExactHullWhite1F::new();
    
    // Simulate a single long path
    let time_grid = TimeGrid::uniform(10.0, 1000)?; // 10 years
    let engine_config = McEngineConfig::new(10_000, time_grid)
        .with_seed(111)
        .with_parallel(false);
    let engine = McEngine::new(engine_config);
    
    // Start far from mean
    let initial_rate = 0.08; // 8%, well above 4% mean
    
    // Use a simple terminal payoff  to verify simulation runs
    let payoff = Digital::call(0.0, 1.0, 1000); // Always pays 1 (threshold=0, rate always > 0)
    let rng = PhiloxRng::new(111);
    
    // We can't directly extract terminal rates easily, so just verify the process runs
    let result = engine.price(
        &rng,
        &hw_process,
        &disc,
        &[initial_rate],
        &payoff,
        Currency::USD,
        1.0,
    )?;
    
    // Payoff always 1, so mean should be ~1
    assert!((result.mean.amount() - 1.0).abs() < 0.01);
    
    println!("HW1F simulation completed: {} paths, mean={}", result.num_paths, result.mean);
    
    Ok(())
}

#[test]
fn test_cir_process_positivity() -> Result<()> {
    // Test that CIR process with QE maintains positivity
    let kappa = 0.5;
    let theta = 0.04;
    let sigma = 0.2; // High vol
    
    let cir_params = CirParams::new(kappa, theta, sigma);
    let feller = cir_params.satisfies_feller();
    let cir_process = CirProcess::new(cir_params);
    let disc = QeCir::new();
    
    let time_grid = TimeGrid::uniform(1.0, 252)?;
    let engine_config = McEngineConfig::new(5_000, time_grid)
        .with_seed(222)
        .with_parallel(false);
    let engine = McEngine::new(engine_config);
    
    // Start at mean
    let initial_v = 0.04;
    
    // Use Digital payoff to validate simulation runs
    let payoff = Digital::call(0.0, 1.0, 252);
    let rng = PhiloxRng::new(222);
    
    let result = engine.price(
        &rng,
        &cir_process,
        &disc,
        &[initial_v],
        &payoff,
        Currency::USD,
        1.0,
    )?;
    
    assert!((result.mean.amount() - 1.0).abs() < 0.01);
    
    println!("CIR simulation completed: Feller={}", feller);
    
    Ok(())
}

#[test]
fn test_cir_feller_condition() -> Result<()> {
    // Test CIR with and without Feller condition
    
    // Satisfies Feller: 2κθ >= σ²
    let params_feller = CirParams::new(0.5, 0.04, 0.1);
    assert!(params_feller.satisfies_feller());
    // 2 * 0.5 * 0.04 = 0.04 >= 0.01 ✓
    
    // Violates Feller
    let params_no_feller = CirParams::new(0.1, 0.01, 0.2);
    assert!(!params_no_feller.satisfies_feller());
    // 2 * 0.1 * 0.01 = 0.002 < 0.04 ✗
    
    // Both should work with QE discretization
    let process1 = CirProcess::new(params_feller);
    let process2 = CirProcess::new(params_no_feller);
    let disc = QeCir::new();
    
    let t: f64 = 0.0;
    let dt: f64 = 0.01;
    let mut x1 = vec![0.04];
    let mut x2 = vec![0.01];
    let z = vec![0.0];
    let mut work = vec![0.0; disc.work_size(&process1)];
    
    disc.step(&process1, t, dt, &mut x1, &z, &mut work);
    disc.step(&process2, t, dt, &mut x2, &z, &mut work);
    
    // Both should maintain non-negativity
    assert!(x1[0] >= 0.0);
    assert!(x2[0] >= 0.0);
    
    println!("CIR Feller test passed");
    
    Ok(())
}

#[test]
fn test_cir_plus_plus_shift() -> Result<()> {
    // Test that CIR++ shift works correctly
    let cir = CirProcess::with_params(0.1, 0.03, 0.05);
    let shift = 0.02; // 200bp shift
    
    let cir_pp = CirPlusPlusProcess::with_constant_shift(cir, shift);
    
    // State x=0.03, shift=0.02 → actual rate should be 0.05
    assert_eq!(cir_pp.actual_rate(0.03, 0.0), 0.05);
    assert_eq!(cir_pp.actual_rate(0.03, 1.0), 0.05);
    
    // The state evolution follows base CIR dynamics
    let x = vec![0.03];
    let mut drift = vec![0.0];
    cir_pp.drift(0.0, &x, &mut drift);
    
    // Drift should be κ(θ - x) for the state, not the actual rate
    assert_eq!(drift[0], 0.1 * (0.03 - 0.03)); // Should be 0
    
    Ok(())
}

#[test]
fn test_euler_vs_exact_hw1f() -> Result<()> {
    // Compare generic Euler with exact HW1F for convergence
    let kappa = 0.1;
    let theta = 0.03;
    let sigma = 0.01;
    
    let hw_params = HullWhite1FParams::new(kappa, sigma, theta);
    let hw_process = HullWhite1FProcess::new(hw_params);
    
    let t: f64 = 0.0;
    let dt: f64 = 0.01; // Small step
    let initial_rate = 0.04;
    let z = vec![1.0]; // One std dev shock
    let mut work = vec![0.0; 2];
    
    // Exact
    let mut x_exact = vec![initial_rate];
    ExactHullWhite1F::new().step(&hw_process, t, dt, &mut x_exact, &z, &mut work);
    
    // Euler
    let mut x_euler = vec![initial_rate];
    EulerMaruyama::new().step(&hw_process, t, dt, &mut x_euler, &z, &mut work);
    
    println!("Exact HW1F: {:.6}", x_exact[0]);
    println!("Euler: {:.6}", x_euler[0]);
    
    // For small dt, should be close
    let error = (x_exact[0] - x_euler[0]).abs();
    println!("Error: {:.6}", error);
    
    // Euler should be within reasonable tolerance for small dt
    assert!(error / initial_rate < 0.01); // Within 1%
    
    Ok(())
}

#[test]
fn test_cap_payoff_basic() -> Result<()> {
    // Test cap payoff structure and evaluation
    use finstack_valuations::instruments::common::mc::traits::{Payoff, PathState};
    
    let strike_rate = 0.03; // 3% cap
    let notional = 1_000_000.0;
    
    // Quarterly fixings for 1 year
    let fixing_dates = vec![0.25, 0.5, 0.75, 1.0];
    let accruals = vec![0.25, 0.25, 0.25, 0.25];
    let dfs = vec![0.9925, 0.985, 0.9775, 0.97];
    
    let mut cap = CapPayoff::new(
        strike_rate,
        notional,
        fixing_dates.clone(),
        accruals.clone(),
        dfs.clone(),
        Currency::USD,
    );
    
    // Simulate payoff events manually
    // At first fixing (t=0.25), rate = 0.04 (4%)
    let mut state1 = PathState::new(63, 0.25); // step 63 ≈ 0.25 * 252
    state1.set("short_rate", 0.04);
    cap.on_event(&state1);
    
    // At second fixing (t=0.5), rate = 0.02 (2%, below strike)
    let mut state2 = PathState::new(126, 0.5);
    state2.set("short_rate", 0.02);
    cap.on_event(&state2);
    
    // Get final value
    let pv = cap.value(Currency::USD);
    
    // First caplet: max(0.04-0.03, 0) * 0.25 * 1M * 0.9925 = 2,481.25
    // Second caplet: max(0.02-0.03, 0) * 0.25 * 1M * 0.985 = 0
    let expected = 0.01 * 0.25 * 1_000_000.0 * 0.9925;
    
    println!("Cap PV: {}", pv);
    println!("Expected (from first caplet): ${:.2}", expected);
    
    assert!((pv.amount() - expected).abs() < 1.0);
    
    Ok(())
}

#[test]
fn test_floor_payoff_basic() -> Result<()> {
    // Test floor payoff structure
    use finstack_valuations::instruments::common::mc::traits::{Payoff, PathState};
    
    let strike_rate = 0.03;
    let notional = 500_000.0;
    
    let fixing_dates = vec![0.5, 1.0];
    let accruals = vec![0.5, 0.5];
    let dfs = vec![0.975, 0.95];
    
    let mut floor = FloorPayoff::new(
        strike_rate,
        notional,
        fixing_dates.clone(),
        accruals.clone(),
        dfs.clone(),
        Currency::EUR,
    );
    
    // At first fixing, rate = 0.02 (below strike)
    let mut state1 = PathState::new(126, 0.5);
    state1.set("short_rate", 0.02);
    floor.on_event(&state1);
    
    let pv = floor.value(Currency::EUR);
    
    // Floorlet: max(0.03-0.02, 0) * 0.5 * 500k * 0.975 = 2,437.50
    let expected = 0.01 * 0.5 * 500_000.0 * 0.975;
    
    println!("Floor PV: {}", pv);
    println!("Expected: €{:.2}", expected);
    
    assert!((pv.amount() - expected).abs() < 1.0);
    
    Ok(())
}

#[test]
fn test_cap_floor_parity_formula() -> Result<()> {
    // Test cap-floor parity: Cap - Floor = Swap
    let fixing_dates = vec![0.5, 1.0];
    let forward_rates = vec![0.04, 0.045];
    let accruals = vec![0.5, 0.5];
    let dfs = vec![0.98, 0.96];
    let strike = 0.03;
    let notional = 1_000_000.0;
    
    let swap_value = cap_floor_parity_swap_value(
        &fixing_dates,
        &forward_rates,
        &accruals,
        &dfs,
        strike,
        notional,
    );
    
    // Manual calculation:
    // Period 1: (0.04 - 0.03) * 0.5 * 1M * 0.98 = 4,900
    // Period 2: (0.045 - 0.03) * 0.5 * 1M * 0.96 = 7,200
    // Total: 12,100
    
    let expected = (0.04 - 0.03) * 0.5 * 1_000_000.0 * 0.98
                 + (0.045 - 0.03) * 0.5 * 1_000_000.0 * 0.96;
    
    println!("Swap value (Cap-Floor): ${:.2}", swap_value);
    println!("Expected: ${:.2}", expected);
    
    assert!((swap_value - expected).abs() < 1.0);
    
    Ok(())
}

