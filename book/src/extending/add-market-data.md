# Add Market Data

This guide covers adding a new market data type (curve, surface, or scalar).

## Step 1: Define the Rust Type

Add in `finstack/core/src/market_data/`:

```rust,no_run
/// My new market data type.
#[derive(Debug, Clone)]
pub struct MyNewCurve {
    /// Curve identifier.
    id: String,
    /// Base date for the curve.
    base_date: Date,
    /// Knot points.
    knots: Vec<(f64, f64)>,
}
```

## Step 2: Implement Core Methods

```rust,no_run
impl MyNewCurve {
    /// Interpolate the curve at time t.
    pub fn value(&self, t: f64) -> Result<f64> {
        // Use the math interpolation utilities
        interp::interpolate(&self.knots, t, self.interp_style)
    }

    /// Batch evaluation for performance.
    pub fn value_batch(&self, times: &[f64]) -> Result<Vec<f64>> {
        times.iter().map(|&t| self.value(t)).collect()
    }
}
```

## Step 3: Add a Builder

```rust,no_run
impl MyNewCurve {
    pub fn builder(id: impl Into<String>) -> MyNewCurveBuilder {
        MyNewCurveBuilder::new(id)
    }
}

pub struct MyNewCurveBuilder {
    id: String,
    base_date: Option<Date>,
    knots: Vec<(f64, f64)>,
    interp: InterpStyle,
}
```

## Step 4: Register in MarketContext

Add storage and retrieval in `MarketContext`:

```rust,no_run
impl MarketContext {
    pub fn add_my_curve(&mut self, curve: MyNewCurve) {
        self.my_curves.insert(curve.id().to_string(), curve);
    }

    pub fn get_my_curve(&self, id: &str) -> Result<&MyNewCurve> {
        self.my_curves.get(id)
            .ok_or_else(|| Error::CurveNotFound(id.to_string()))
    }
}
```

## Step 5: Python Binding

Follow the [Add a Python Binding](add-python-binding.md) guide to create
the PyO3 wrapper.

## Step 6: Tests

1. Unit test: Construction, interpolation, edge cases
2. Integration test: Use in a pricer via MarketContext
3. Parity test: Python matches Rust

## Calibration (Optional)

If the curve needs calibration from market instruments:

```rust,no_run
pub fn calibrate_my_curve(
    market_quotes: &[(String, f64)],  // instrument ID, market price
    disc: &DiscountCurve,
) -> Result<MyNewCurve> {
    // Use Newton or Brent solver from finstack_core::math
    let solver = NewtonSolver::new(tolerance, max_iter);
    // Bootstrap knots sequentially
    // ...
}
```
