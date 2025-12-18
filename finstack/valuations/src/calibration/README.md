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
- `api/`: Defines the structured calibration schema (V2) and execution engine.
- `solver/`: Contains core numerical solvers (Sequential Bootstrap, Levenberg-Marquardt).
- `pricing/`: Infrastructure for pricing instruments during calibration.
- `quotes/`: Market quote types and extraction logic.
- `adapters/`: Logic for mapping abstract calibration steps to concrete execution.
- `validation/`: Runtime validation of calibrated structures.
- `bumps/`: Support for re-calibration and risk sensitivities.

## Usage Examples

### Executing a Calibration Plan

```rust
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelopeV2, CalibrationPlanV2, CALIBRATION_SCHEMA_V2,
};

fn run_calibration(plan: CalibrationPlanV2) -> finstack_core::Result<()> {
    let envelope = CalibrationEnvelopeV2 {
        schema: CALIBRATION_SCHEMA_V2.to_string(),
        plan,
        initial_market: None,
    };
    
    let result = engine::execute(&envelope)?;
    println!("Calibrated {} structures", result.calibrated_structures.len());
    Ok(())
}
```

## Adding New Features

### Adding a New Calibration Target
1. Implement the `BootstrapTarget` trait in `solver/traits.rs` (if using bootstrapping).
2. Implement the `GlobalSolveTarget` trait (if using global optimization).
3. Register the new target in the `api` engine and `adapters`.

### Adding a New Instrument Type
1. Define the instrument's quote type in `quotes/`.
2. Implement a `CalibrationPricer` in `pricing/`.
3. Update `quote_factory` to support extracting the new instrument.

## Performance and Reliability

- **No-alloc Hot Loops**: Solvers are designed to minimize heap allocations during iteration.
- **Deterministic**: Calibration results are deterministic given the same inputs and configuration.
- **Strict Validation**: Optional strict mode ensures all conventions are explicitly defined.
