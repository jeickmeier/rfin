# Calibration Module

The `calibration` module provides a high-performance, plan-driven framework for calibrating market data structures (curves, surfaces, correlations) from liquid market instruments.

## Functionality

This module supports:

- **Interest Rate Curves**: Discount and forward curves using OIS, swaps, futures, and fra.
- **Credit Curves**: Survival and hazard rate curves from CDS and credit indices.
- **Inflation Curves**: Inflation-indexed curves.
- **Volatility Surfaces**: SABR and other volatility models.
- **Base Correlation**: For credit tranches.

## Structure

The module is organized into several key areas:

- `api/`: Defines the structured calibration schema and execution engine.
- `solver/`: Contains core numerical solvers (Sequential Bootstrap, Levenberg-Marquardt).
- `targets/`: Core logic for instrument-specific calibration targets (Bootstrappers).
- `prepared.rs`: Internal calibration quote envelopes (wrapping market-level quotes).
- `validation/`: Runtime validation of calibrated structures.
- `bumps/`: Support for re-calibration and risk sensitivities.

## Usage Examples

### Executing a Calibration Plan

```rust
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan, CALIBRATION_SCHEMA,
};

fn run_calibration(plan: CalibrationPlan) -> finstack_core::Result<()> {
    let envelope = CalibrationEnvelope {
        schema: CALIBRATION_SCHEMA.to_string(),
        plan,
        initial_market: None,
    };

    let result = engine::execute(&envelope)?;
    println!("Calibrated {} structures", result.calibrated_structures.len());
    Ok(())
}
```

## Configuration Guide

### Tolerance Semantics

Calibration involves two distinct tolerance concepts that control different aspects:

1. **Solver Tolerance** (`config.solver.tolerance()`):
   - Controls when the numerical solver (Brent/Newton) terminates
   - This is an algorithmic convergence criterion in parameter space
   - The solver stops when successive parameter estimates differ by less than this tolerance
   - Default: `1e-12`

2. **Validation Tolerance** (`config.discount_curve.validation_tolerance`, etc.):
   - Controls whether calibration is considered *successful*
   - After the solver converges, final residuals are compared against this tolerance
   - If any residual exceeds `validation_tolerance`, calibration is marked as failed
   - Default: `1e-8` (suitable for per-unit-notional residuals)

**Why two tolerances?**

- Solver tolerance ensures numerical convergence but doesn't guarantee economic fit
- Validation tolerance ensures the calibrated curve actually prices instruments correctly
- For well-behaved problems, solver tolerance of `1e-12` with validation tolerance of `1e-8` works well: the solver finds a precise root, and we verify it prices accurately

### Configuration Hierarchy

Settings can be specified at multiple levels with the following precedence:

1. **Step-level** (`CalibrationStep.params.method`): Per-instrument-type overrides (highest priority)
2. **Plan-level** (`CalibrationPlan.settings`): Plan-wide defaults
3. **Global defaults** (`CalibrationConfig::default()`): Fallback values

Step-level settings always take precedence over plan-level settings. For example:

```rust
// Plan-level default: Bootstrap
let plan = CalibrationPlan {
    settings: CalibrationConfig::default(), // Uses Bootstrap by default
    steps: vec![
        CalibrationStep {
            // Step-level override: GlobalSolve for this specific curve
            params: StepParams::Discount(DiscountCurveParams {
                method: CalibrationMethod::GlobalSolve { use_analytical_jacobian: true },
                ..
            }),
            ..
        }
    ],
    ..
};
```

### Recommended Settings

| Use Case | Solver Tolerance | Validation Tolerance | Method |
|----------|------------------|---------------------|--------|
| Production risk systems | `1e-12` | `1e-8` | Bootstrap |
| Real-time pricing | `1e-6` | `1e-4` | Bootstrap |
| Interactive exploration | `1e-4` | `1e-2` | Bootstrap |
| Smooth curve fitting | `1e-10` | `1e-8` | GlobalSolve |
| Distressed credit | `1e-10` | `1e-6` | Bootstrap |

## Adding New Features

### Adding a New Calibration Target

1. Implement the `BootstrapTarget` trait in `solver/` (if using bootstrapping).
2. Create a new target/bootstrapper in `targets/`.
3. Register the new target in the `api` engine and `targets/handlers.rs`.

### Adding a New Instrument Type

1. Define the instrument's quote type in `market/quotes/`.
2. Update the `targets/` logic to support building and pricing the new instrument.

## Performance and Reliability

- **Allocation-aware Hot Loops**: Solver inner iterations (Brent / LM residual evaluation) reuse buffers via `RefCell<Vec<_>>` and avoid heap allocations. Per-knot reporting allocates one small `String` per residual key, which is `O(n_quotes)` not `O(n_iters × n_quotes)` — negligible relative to solver cost.
- **Deterministic**: Calibration results are deterministic given the same inputs and configuration. Multi-start uses Halton sequences (no system RNG); residual maps use `BTreeMap` for stable ordering.
- **Strict Validation**: Optional strict mode ensures all conventions are explicitly defined.
- **Production Diagnostics**: When a global-solve calibration fails, the `convergence_reason` includes the top-3 worst-fit quotes inline (no need to re-run with `compute_diagnostics=true` to know which instruments drove the failure). For full per-quote sensitivity / condition number, set `config.compute_diagnostics = true`.
