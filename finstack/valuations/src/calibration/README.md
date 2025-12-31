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

## Adding New Features

### Adding a New Calibration Target

1. Implement the `BootstrapTarget` trait in `solver/` (if using bootstrapping).
2. Create a new target/bootstrapper in `targets/`.
3. Register the new target in the `api` engine and `targets/handlers.rs`.

### Adding a New Instrument Type

1. Define the instrument's quote type in `market/quotes/`.
2. Update the `targets/` logic to support building and pricing the new instrument.

## Performance and Reliability

- **No-alloc Hot Loops**: Solvers are designed to minimize heap allocations during iteration.
- **Deterministic**: Calibration results are deterministic given the same inputs and configuration.
- **Strict Validation**: Optional strict mode ensures all conventions are explicitly defined.
