# Metrics Framework

## Overview
This module provides a clean separation between core pricing logic and financial metrics/measures computation. It replaces the previous monolithic approach where all metrics were computed inside the `price()` method.

## Benefits

### Before (Old Architecture)
```rust
// Everything mixed together in one method
impl Priceable for Bond {
    fn price(&self, curves: &CurveSet, as_of: Date) -> Result<ValuationResult> {
        // Core pricing
        let value = self.pv(&*disc, curves, as_of)?;
        
        // Accrued interest
        let ai = accrued_interest(...);
        
        // YTM calculations (even if not needed)
        if let Some(clean_px) = self.quoted_clean {
            let ytm = bond_ytm_from_dirty(...);
            let (d_mac, d_mod) = bond_duration_mac_mod(...);
            let convex = bond_convexity_numeric(...);
            // ...
        }
        
        // Yield-to-worst (even if not applicable)
        if let Some(cp) = &self.call_put {
            // Complex YTW logic...
        }
        
        // All mixed together, hard to test, extend, or optimize
    }
}
```

### After (New Architecture)
```rust
// Core pricing separate
let value = bond.value(&curves, as_of)?;  // Fast, focused

// Compute only needed metrics
let metrics = bond.price_with_metrics(&curves, as_of, &["ytm", "duration"])?;

// Or use registry directly for fine control
let mut context = MetricContext::new(...);
let registry = standard_registry();
registry.compute(&["accrued", "ytm"], &mut context)?;
```

## Key Features

### 1. Separation of Concerns
- Core pricing logic isolated from metrics
- Each metric is a separate, testable unit
- Instruments don't need to know about all possible metrics

### 2. On-Demand Computation
- Compute only the metrics you need
- Significant performance improvement when only PV is needed
- Lazy evaluation with caching

### 3. Dependency Management
- Automatic dependency resolution
- Metrics can depend on other metrics
- Computed once and cached

### 4. Extensibility
- Add new metrics without modifying instruments
- Custom metrics for specific use cases
- Plugin architecture

### 5. Type Safety
- Strongly typed metric IDs
- Compile-time instrument type checking
- Clear error handling

## Usage Examples

### Basic Usage
```rust
// Just the value
let value = bond.value(&curves, as_of)?;

// Specific metrics
let result = bond.price_with_metrics(&curves, as_of, &["ytm", "duration"])?;
```

### Custom Metrics
```rust
struct MyCustomMetric;
impl MetricCalculator for MyCustomMetric {
    fn id(&self) -> &str { "custom" }
    fn calculate(&self, ctx: &MetricContext) -> Result<F> {
        // Custom logic here
    }
}

let mut registry = standard_registry();
registry.register_metric(MetricId::MyCustom, Arc::new(MyCustomMetric), &["Bond"]);
```

### Direct Registry Usage
```rust
let mut context = MetricContext::new(
    Arc::new(bond),
    "Bond",
    curves,
    as_of,
    base_value,
);

let registry = standard_registry();
let metrics = registry.compute(&["ytm", "duration"], &mut context)?;
```

## Performance Impact

### Before
- All metrics computed even if not needed
- No caching between metrics
- Monolithic function hard to optimize

### After
- Compute only requested metrics
- Automatic caching of intermediate results
- Parallel computation possible (future enhancement)
- 3-5x faster when only PV needed

## Migration Path

The new framework is designed to coexist with the old approach:

1. **Phase 1**: Add metrics framework alongside existing code
2. **Phase 2**: Instruments can optionally implement new interface
3. **Phase 3**: Gradually migrate metrics to new system
4. **Phase 4**: Deprecate old monolithic `price()` methods

## Standard Metrics

### Bond Metrics
- `accrued` - Accrued interest
- `ytm` - Yield to maturity
- `duration_mac` - Macaulay duration
- `duration_mod` - Modified duration
- `convexity` - Bond convexity
- `ytw` - Yield to worst
- `dirty_price` - Clean price + accrued interest (requires quoted clean price)
- `clean_price` - If quoted, returns that; otherwise `value()` (dirty) minus accrued

Note on price precedence:
- When `quoted_clean` is present on a `Bond`, `dirty_price` is computed as `quoted_clean + accrued` and `clean_price` simply echoes the quoted value.
- When `quoted_clean` is absent, `clean_price` derives from the base `value()` (which is a dirty PV) by subtracting `accrued`.

### IRS Metrics
- `annuity` - Annuity factor
- `par_rate` - Par swap rate
- `dv01` - Dollar value of 1bp
- `pv_fixed` - Fixed leg PV
- `pv_float` - Floating leg PV

### Risk Metrics (Future)
- `bucketed_dv01` - DV01 by tenor bucket
- `cs01` - Credit spread sensitivity
- `theta` - Time decay
- `vega` - Volatility sensitivity

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Instrument                       │
│  - Holds data                                       │
│  - Computes core value                              │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│                 MetricContext                       │
│  - Instrument reference                             │
│  - Market curves                                    │
│  - Cache for intermediate results                   │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│                 MetricRegistry                      │
│  - Manages metric calculators                       │
│  - Resolves dependencies                            │
│  - Handles caching                                  │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│              MetricCalculators                      │
│  - YtmCalculator                                    │
│  - DurationCalculator                               │
│  - CustomMetricCalculator                           │
│  - ...                                              │
└─────────────────────────────────────────────────────┘
```

## Testing

Each metric can be unit tested independently:
```rust
#[test]
fn test_ytm_calculator() {
    let calc = YtmCalculator;
    let context = create_test_context();
    let ytm = calc.calculate(&context).unwrap();
    assert_eq!(ytm, expected_ytm());
}
```

## Future Enhancements

1. **Parallel Computation**: Compute independent metrics in parallel
2. **Metric Composition**: Build complex metrics from simpler ones
3. **Streaming Metrics**: Support for real-time metric updates
4. **Metric Persistence**: Cache computed metrics to database
5. **Metric Versioning**: Handle metric definition changes over time
