# Using Analytic Derivatives in Calibration

## Overview

As of this update, `finstack_core::math::solver::NewtonSolver` supports an optional analytic derivative API via `solve_with_derivative()`. This document outlines the recommended pattern for adopting analytic derivatives in calibration and risk calculations within the valuations crate.

## Motivation

**Performance Benefits:**
- 2× fewer function evaluations (no need for finite-difference `f(x±h)`)
- Faster convergence due to exact derivatives
- Better numerical stability (avoids finite-difference cancellation errors)

**When to Use:**
- Implied volatility calculation (Vega is available from option pricing)
- Yield-to-maturity solving (Duration/DV01 available from bond pricing)
- Spread solving (CS01 available from credit instruments)
- Any calibration where sensitivities are already computed

## Core API

### NewtonSolver with Analytic Derivatives

```rust
use finstack_core::math::solver::NewtonSolver;

let solver = NewtonSolver::new()
    .with_tolerance(1e-6)
    .with_max_iterations(100);

// Old way (still supported):
// solver.solve(f, initial_guess)

// New way with analytic derivative:
solver.solve_with_derivative(f, f_prime, initial_guess)
```

Where:
- `f: impl Fn(f64) -> f64` — objective function (e.g., `price(vol) - market_price`)
- `f_prime: impl Fn(f64) -> f64` — derivative of `f` with respect to the solve variable (e.g., Vega)
- `initial_guess: f64` — starting point

## Recommended Adoption Pattern

### Phase 1: Core Call Sites (Completed)

✅ XIRR and IRR now use analytic derivatives (NPV derivatives with respect to rate)

### Phase 2: Calibration Metrics (Incremental Adoption)

For metrics like implied volatility, the pattern is:

#### Before (Finite Difference)

```rust
use finstack_core::math::solver::{BrentSolver, Solver};

let objective = |vol: f64| {
    let price = price_instrument(vol, ...);
    price - market_price
};

let solver = BrentSolver::new().with_tolerance(1e-6);
let implied_vol = solver.solve(objective, 0.20)?;
```

#### After (Analytic Derivative)

```rust
use finstack_core::math::solver::{NewtonSolver, Solver};

let objective = |vol: f64| {
    let price = price_instrument(vol, ...);
    price - market_price
};

// Vega: ∂Price/∂σ
let derivative = |vol: f64| {
    vega_instrument(vol, ...)
};

let solver = NewtonSolver::new().with_tolerance(1e-6);
let implied_vol = solver.solve_with_derivative(objective, derivative, 0.20)?;
```

**Key Points:**
- Only adopt where the derivative (e.g., Vega, Duration, CS01) is **already computed** or **cheap to compute analytically**
- If computing the derivative is expensive, keep using finite differences
- Wrap in a try-catch pattern with fallback to Brent if needed for robustness

### Phase 3: Curve Calibration (Future Work)

For discount curve, hazard curve, and volatility surface calibration:

1. **Single-node calibration** (e.g., bootstrapping):
   - Objective: `price(node_rate) - market_price = 0`
   - Derivative: DV01 or CS01 with respect to the calibrated node
   - Use `NewtonSolver::solve_with_derivative` directly

2. **Multi-node calibration** (simultaneous):
   - Use `finstack_core::math::solver_multi` for Levenberg-Marquardt with Jacobians
   - Pattern already exists in `sabr_derivatives.rs` for reference

## Integration with SolverConfig

The `SolverConfig` enum in `calibration/solver_config.rs` currently supports Newton, Brent, and Hybrid. When using `solve_with_derivative`, you can:

1. **Keep SolverConfig as-is**: Use it to configure the Newton instance, then call `solve_with_derivative` directly
2. **Extend SolverConfig (future)**: Add a flag indicating analytic derivatives are available (optional enhancement)

Example usage:

```rust
// Build solver from config
let newton = match &config {
    SolverConfig::Newton { tolerance, max_iterations, .. } => {
        NewtonSolver::new()
            .with_tolerance(*tolerance)
            .with_max_iterations(*max_iterations)
    },
    _ => NewtonSolver::default(),
};

// Use with analytic derivative
let result = newton.solve_with_derivative(objective, derivative, guess)?;
```

## Concrete Examples

### Example 1: Implied Volatility (Options)

Location: `instruments/cap_floor/metrics/implied_vol.rs`

**Current (Brent):**
- Solver: `BrentSolver::solve(objective, 0.20)`
- Function evals: ~20-30 (including bracketing)

**Proposed (Newton with Vega):**
```rust
// Assume we have a vega function available
let vega = |vol: f64| {
    let inputs = CapletFloorletInputs { volatility: vol, ..base_inputs };
    vega_caplet_floorlet(inputs).unwrap_or(0.0)
};

let solver = NewtonSolver::new().with_tolerance(1e-6);
let implied_vol = solver.solve_with_derivative(objective, vega, 0.20)?;
```
- Function evals: ~5-8 (quadratic convergence)

### Example 2: Yield-to-Maturity (Bonds)

Location: `instruments/bond/pricing/ytm_solver.rs`

**Current:** Uses Newton with finite-difference derivatives

**Proposed:** Reuse already-computed duration (modified duration = -∂P/∂y / P):
```rust
let price_error = |ytm: f64| price_at_ytm(ytm) - market_price;

// Duration available from bond pricing
let price_derivative = |ytm: f64| {
    let price = price_at_ytm(ytm);
    let duration = modified_duration_at_ytm(ytm);
    -price * duration  // ∂Price/∂ytm
};

solver.solve_with_derivative(price_error, price_derivative, 0.05)?;
```

### Example 3: Credit Spread Calibration

For CDS calibration, CS01 (∂Price/∂Spread) is already computed in risk metrics:

```rust
let spread_error = |spread: f64| cds_price(spread, ...) - market_price;
let cs01 = |spread: f64| compute_cs01(spread, ...);

solver.solve_with_derivative(spread_error, cs01, 0.01)?;
```

## Testing and Validation

When adopting analytic derivatives:

1. **Parity Test**: Verify that Newton with analytic derivatives produces the same result as Brent (within tolerance)
2. **Performance Benchmark**: Measure iteration count and wall-clock time vs finite-difference
3. **Edge Cases**: Test near boundaries (e.g., vol → 0, spread → 0) where derivatives might be undefined

Example test pattern:

```rust
#[test]
fn test_implied_vol_newton_vs_brent() {
    let brent_vol = solve_with_brent(...);
    let newton_vol = solve_with_newton_analytic(...);
    
    assert!((brent_vol - newton_vol).abs() < 1e-6);
    
    // Optional: check iteration counts
    // assert!(newton_iterations < brent_iterations);
}
```

## Migration Strategy

**Prioritization (High → Low Impact):**

1. ✅ **Core cashflow metrics** (XIRR, IRR) — **Completed**
2. **Implied volatility** (options) — High-frequency calculation, many strikes
3. **Yield-to-maturity / Z-spread** (bonds) — Common in portfolio valuation
4. **Credit spread calibration** (CDS) — Critical for credit risk
5. **Curve bootstrapping** (single-node) — Medium benefit
6. **Volatility surface fitting** (multi-node) — Already has multi-dimensional solver support

**Timeline:**
- Phase 1 (XIRR/IRR): ✅ Complete
- Phase 2 (Implied vols, YTM): Next iteration (incremental, file-by-file)
- Phase 3 (Curve calibration): As needed based on performance profiling

## Performance Expectations

Based on typical Newton convergence:

| Use Case | Finite Difference | Analytic Derivative | Speedup |
|----------|------------------|---------------------|---------|
| XIRR | ~8-12 iterations | ~4-6 iterations | ~2x |
| Implied Vol (options) | Brent ~25 evals | Newton ~5-8 evals | ~3-5x |
| YTM (bonds) | ~10 iterations (2× evals) | ~5 iterations | ~2-4x |

Note: Speedups assume derivative computation is cheap (comparable to or less than function evaluation cost).

## References

- Core API: `finstack/core/src/math/solver.rs` — `NewtonSolver::solve_with_derivative`
- Example usage: `finstack/core/src/cashflow/xirr.rs` — XIRR with analytic NPV derivative
- Multi-dimensional: `finstack/valuations/src/calibration/derivatives/sabr_derivatives.rs` — SABR with Jacobian

## Summary

The analytic derivative API is **opt-in** and **backward-compatible**. Adoption should be incremental, focusing on high-frequency calculations where sensitivities are already available. For cases where derivatives are expensive or unavailable, the existing finite-difference solvers remain the appropriate choice.

