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

// Compute only needed metrics (strongly-typed IDs)
use crate::metrics::MetricId;
let result = bond.price_with_metrics(&curves, as_of, &[MetricId::Ytm, MetricId::DurationMod])?;

// Or use registry directly for fine control
use crate::instruments::Instrument;
use crate::metrics::{MetricContext, standard_registry, MetricId};
use std::sync::Arc;

let mut context = MetricContext::new(
    Arc::new(Instrument::Bond(bond.clone())),
    Arc::new(curves.clone()),
    as_of,
    value,
);

let registry = standard_registry();
registry.compute(&[MetricId::Accrued, MetricId::Ytm], &mut context)?;
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
- Strongly-typed metric IDs via `MetricId`
- Applicability enforced per instrument type via the registry
- Clear error handling

## Metric IDs: Single Source of Truth

Metric IDs are defined with a macro in `metrics/ids.rs` to avoid duplication. Adding a metric in one place updates the enum, string mapping, parser, and standard list automatically.

What the macro generates:
- `enum MetricId` variants
- `MetricId::as_str()` string names (snake_case)
- `impl FromStr` parsing (case-insensitive, snake_case)
- `MetricId::ALL_STANDARD` list

Add a metric by appending a single line in the macro invocation:

```rust
// metrics/ids.rs
define_metrics! {
    // ...existing metrics...
    NewMetric => "new_metric",
}
```

Guidelines:
- Use concise snake_case IDs in quotes (e.g., `"par_rate"`).
- Provide a brief doc comment above each line for docs and IDE hovers.
- For non-standard/user-defined metrics at runtime, use `MetricId::custom("...")`.

## Usage Examples

### Basic Usage
```rust
// Just the value
let value = bond.value(&curves, as_of)?;

// Specific metrics
use crate::metrics::MetricId;
let result = bond.price_with_metrics(&curves, as_of, &[MetricId::Ytm, MetricId::DurationMod])?;
```

### Custom Metrics
```rust
struct MyCustomMetric;
impl MetricCalculator for MyCustomMetric {
    fn calculate(&self, _ctx: &mut MetricContext) -> finstack_core::Result<F> {
        // Custom logic here
        Ok(0.0)
    }
}

let mut registry = standard_registry();
registry.register_metric(MetricId::custom("custom"), Arc::new(MyCustomMetric), &["Bond"]);
```

### Direct Registry Usage
```rust
use crate::instruments::Instrument;
use crate::metrics::{MetricContext, MetricId, standard_registry};
use std::sync::Arc;

let mut context = MetricContext::new(
    Arc::new(Instrument::Bond(bond.clone())),
    Arc::new(curves.clone()),
    as_of,
    base_value,
);

let registry = standard_registry();
let metrics = registry.compute(&[MetricId::Ytm, MetricId::DurationMac], &mut context)?;
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

## Integration

- Call `value()` for fast PV only.
- Call `price_with_metrics(curves, as_of, &[MetricId::...])` to compute a targeted set of metrics.
- Call legacy `price()` to compute PV plus a standard set of metrics per instrument (delegates to the framework).

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
- Spreads:
  - `z_spread` - Zero-vol spread
  - `oas` - Option-adjusted spread
  - `g_spread` - Govvie spread
  - `asw_spread` - Asset swap spread

Note on price precedence:
- When `quoted_clean` is present on a `Bond`, `dirty_price` is computed as `quoted_clean + accrued` and `clean_price` simply echoes the quoted value.
- When `quoted_clean` is absent, `clean_price` derives from the base `value()` (which is a dirty PV) by subtracting `accrued`.

### IRS Metrics
- `annuity` - Annuity factor
- `par_rate` - Par swap rate
- `dv01` - Dollar value of 1bp
- `pv_fixed` - Fixed leg PV
- `pv_float` - Floating leg PV

### Deposit Metrics
- `yf` - Year fraction
- `df_start` - Discount factor at start date
- `df_end` - Discount factor at end date
- `deposit_par_rate` - Par rate implied by DFs
- `df_end_from_quote` - DF(end) implied by quoted rate
- `quote_rate` - Quoted rate if present

### Risk Metrics
- `cs01` - Parallel credit spread sensitivity
- `ir01` - Parallel yield curve sensitivity
- `bucketed_dv01` - DV01 total across standard tenor buckets
- `bucketed_cs01` - Credit spread risk by bucket
- `theta` - Time decay (placeholder)

### CDS Metrics
- `par_spread` - Par spread for CDS
- `risky_pv01` - Risky PV01
- `protection_leg_pv` - Protection leg PV
- `premium_leg_pv` - Premium leg PV
- `jump_to_default` - Jump-to-default amount
- `expected_loss` - Expected loss
- `default_probability` - Default probability
- `recovery_01` - Recovery rate sensitivity

### Option Metrics
- `delta` - Price sensitivity to underlying
- `gamma` - Delta sensitivity to underlying
- `vega` - Price sensitivity to volatility
- `rho` - Price sensitivity to interest rates
- `implied_vol` - Implied volatility from price
- Additional greeks:
  - `vanna` - Delta sensitivity to volatility
  - `volga` - Vega sensitivity to volatility
  - `veta` - Theta sensitivity to volatility
  - `charm` - Rho sensitivity to volatility
  - `color` - Gamma sensitivity to time
  - `speed` - Gamma sensitivity to underlying

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
│  - AccruedInterest/Ytm/Duration/Convexity/Ytw       │
│  - Annuity/ParRate/Dv01/PvFixed/PvFloat (IRS)       │
│  - Yf/DfStart/DfEnd/DepositPar/DfFromQuote/Quote    │
│  - BucketedDv01, Theta (risk)                       │
│  - Custom calculators                                │
└─────────────────────────────────────────────────────┘
```

## Testing

Each metric can be unit tested independently:
```rust
#[test]
fn test_ytm_calculator() {
    let calc = YtmCalculator;
    let mut context = create_test_context();
    let ytm = calc.calculate(&mut context).unwrap();
    assert_eq!(ytm, expected_ytm());
}
```

## Future Enhancements

1. **Parallel Computation**: Compute independent metrics in parallel
2. **Metric Composition**: Build complex metrics from simpler ones
3. **Streaming Metrics**: Support for real-time metric updates
4. **Metric Persistence**: Cache computed metrics to database
5. **Metric Versioning**: Handle metric definition changes over time
