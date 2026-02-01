# Metrics Framework

A trait-based architecture for computing financial metrics independently from core pricing logic. The metrics framework provides clean separation between instrument pricing and risk/analytical measure calculation, with built-in dependency management and caching.

## Overview

The metrics framework enables on-demand computation of financial measures (PV, DV01, Greeks, spreads, etc.) with:

- **Trait-based design**: Generic `MetricCalculator` trait for extensibility
- **Dependency management**: Automatic computation ordering based on metric dependencies
- **Efficient caching**: Reuse of intermediate results (cashflows, discount factors, base valuations)
- **Instrument-specific registration**: Metrics can be registered for specific instrument types
- **Standard registry**: Pre-configured registry with common financial metrics

## Directory Structure

```
metrics/
├── README.md                    # This file
├── mod.rs                       # Public API and standard registry
├── core/                        # Core infrastructure
│   ├── mod.rs                   # Core module exports
│   ├── ids.rs                   # Strongly-typed metric identifiers (MetricId)
│   ├── traits.rs                # MetricCalculator trait and MetricContext
│   ├── registry.rs              # MetricRegistry for calculator management
│   ├── registration_macro.rs   # Convenience macros for registration
│   └── finite_difference.rs    # FD utilities and standard bump sizes
└── sensitivities/               # Sensitivity metrics (risk)
    ├── mod.rs                   # Sensitivity module exports
    ├── dv01.rs                  # Interest rate sensitivity (DV01)
    ├── cs01.rs                  # Credit spread sensitivity (CS01)
    ├── vega.rs                  # Volatility sensitivity (Vega)
    ├── theta.rs                 # Time decay (Theta)
    ├── fd_greeks.rs             # Generic finite difference Greeks
    └── tests/                   # Sensitivity metric tests
```

## Key Features

### 1. Trait-Based Architecture

All metrics implement the `MetricCalculator` trait:

```rust
pub trait MetricCalculator: Send + Sync {
    /// Computes the metric value based on the provided context
    fn calculate(&self, context: &mut MetricContext) -> Result<f64>;

    /// Lists metric IDs this calculator depends on
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
```

### 2. Strongly-Typed Metric IDs

All metrics are identified by the `MetricId` type, which provides:

- Compile-time validation
- Autocomplete support
- Safe refactoring when metric names change

```rust
// Standard metrics are constants
let dv01_id = MetricId::Dv01;
let theta_id = MetricId::Theta;

// Custom metrics supported too
let custom_id = MetricId::custom("my_custom_metric");
```

### 3. Dependency Management

The registry automatically resolves dependencies and computes metrics in the correct order:

```rust
struct MacaulayDuration;

impl MetricCalculator for MacaulayDuration {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Can access previously computed YTM
        let ytm = context.computed.get(&MetricId::Ytm)
            .ok_or(Error::Missing)?;

        // Use YTM to compute duration
        // ...
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]  // YTM will be computed first
    }
}
```

### 4. Efficient Caching

The `MetricContext` caches intermediate results to avoid redundant calculations:

```rust
pub struct MetricContext {
    pub instrument: Arc<dyn Instrument>,
    pub curves: Arc<MarketContext>,
    pub as_of: Date,
    pub base_value: Money,
    pub computed: HashMap<MetricId, f64>,           // Scalar metrics
    pub computed_series: HashMap<MetricId, Vec<(String, f64)>>,  // 1D bucketed
    pub computed_matrix: HashMap<MetricId, Structured2D>,        // 2D bucketed
    pub computed_tensor3: HashMap<MetricId, Structured3D>,       // 3D bucketed
    pub cashflows: Option<Vec<(Date, Money)>>,     // Cached cashflows
    // ... other cached data
}
```

### 5. Bucketed Metrics

Support for multi-dimensional risk metrics:

- **1D bucketed**: Key-rate DV01, CS01 by tenor
- **2D structured**: Vega surface (expiry × strike)
- **3D structured**: Advanced risk grids

```rust
// Store bucketed DV01 by tenor
let buckets = vec![
    ("3m".to_string(), 10.5),
    ("1y".to_string(), 42.3),
    ("5y".to_string(), 125.7),
];
context.store_bucketed_series(MetricId::BucketedDv01, buckets);
```

## How to Add a New Metric

### Step 1: Add Metric ID

Add your metric identifier to `core/ids.rs`:

```rust
impl MetricId {
    // ... existing metrics

    /// Your new metric description
    pub const MyNewMetric: Self = Self(Cow::Borrowed("my_new_metric"));
}
```

Don't forget to add it to the `ALL_STANDARD` array if it's a standard metric:

```rust
pub const ALL_STANDARD: &'static [MetricId] = &[
    // ... existing metrics
    MetricId::MyNewMetric,
];
```

### Step 2: Implement the Calculator

Create a calculator struct that implements `MetricCalculator`:

```rust
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct MyNewMetricCalculator;

impl MetricCalculator for MyNewMetricCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // 1. Downcast instrument if needed
        let bond: &Bond = context.instrument_as()?;

        // 2. Access dependencies
        let ytm = context.computed.get(&MetricId::Ytm)
            .copied()
            .unwrap_or(0.0);

        // 3. Perform calculation
        let result = ytm * bond.face_value().amount();

        Ok(result)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]  // Declare dependencies
    }
}
```

### Step 3: Register the Metric

Add registration in the appropriate instrument's `metrics.rs` module:

```rust
pub fn register_bond_metrics(registry: &mut MetricRegistry) {
    // ... existing registrations

    registry.register_metric(
        MetricId::MyNewMetric,
        Arc::new(MyNewMetricCalculator),
        &["Bond"],  // Applies to Bond only
    );
}
```

Or register for all instruments:

```rust
registry.register_metric(
    MetricId::MyNewMetric,
    Arc::new(MyNewMetricCalculator),
    &[],  // Empty = applies to all instruments
);
```

### Step 4: Add Tests

Create comprehensive tests in the appropriate test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{MetricRegistry, MetricContext};
    use std::sync::Arc;

    #[test]
    fn test_my_new_metric() {
        // Setup
        let bond = create_test_bond();
        let market = create_test_market();
        let as_of = create_date(2024, Month::January, 1).unwrap();

        // Create context
        let base_value = bond.value(&market, as_of).unwrap();
        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        // Calculate metric
        let calculator = MyNewMetricCalculator;
        let result = calculator.calculate(&mut context).unwrap();

        // Assert
        assert!((result - expected).abs() < 1e-6);
    }
}
```

### Step 5: Document the Metric

Add comprehensive documentation to `METRICS.md`:

```markdown
## MyNewMetric

**Category**: Bond Metrics
**Unit**: Dollars
**Sign Convention**: Positive = gains value when X increases

### Definition

[Clear mathematical definition or business explanation]

### Formula

```

MyNewMetric = YTM × Face Value

```

### Example

[Working code example showing usage]

### See Also

- Related metrics
- References to standards or papers
```

## Common Patterns

### Generic Calculators with Type Parameters

For reusable calculators across multiple instrument types:

```rust
use std::marker::PhantomData;

pub struct GenericDv01Calculator<I> {
    _phantom: PhantomData<I>,
}

impl<I: Instrument + CurveDependencies + 'static> MetricCalculator
    for GenericDv01Calculator<I>
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let curve_id = instrument.discount_curve_id();

        // Bump and reprice
        let bumped_market = bump_curve(&context.curves, curve_id, 0.0001)?;
        let bumped_pv = instrument.value(&bumped_market, context.as_of)?;

        let dv01 = (bumped_pv.amount() - context.base_value.amount()) / 10_000.0;
        Ok(dv01)
    }
}
```

### Bucketed/Key-Rate Metrics

For metrics that compute sensitivities across multiple buckets:

```rust
impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let buckets = standard_ir_dv01_buckets();  // [0.25, 0.5, 1.0, ...]
        let mut series = Vec::new();
        let mut total = 0.0;

        for bucket_time in buckets {
            let label = format_bucket_label(bucket_time);

            // Bump at key rate
            let bumped_market = bump_key_rate(
                &context.curves,
                &curve_id,
                bucket_time,
                0.0001
            )?;

            let bumped_pv = instrument.value(&bumped_market, context.as_of)?;
            let bucket_dv01 = (bumped_pv.amount() - context.base_value.amount()) / 10_000.0;

            series.push((label, bucket_dv01));
            total += bucket_dv01;
        }

        // Store bucketed series
        context.store_bucketed_series(MetricId::BucketedDv01, series);

        Ok(total)
    }
}
```

### Metrics with Configuration

For calculators that need configuration:

```rust
pub struct ConfigurableThetaCalculator {
    period: String,  // "1D", "1W", "1M", etc.
}

impl ConfigurableThetaCalculator {
    pub fn new(period: String) -> Self {
        Self { period }
    }

    pub fn daily() -> Self {
        Self::new("1D".to_string())
    }
}

impl MetricCalculator for ConfigurableThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let days = parse_period_days(&self.period)?;
        let forward_date = context.as_of + Duration::days(days);

        // Price at forward date
        let forward_pv = context.instrument.value(&context.curves, forward_date)?;
        let theta = forward_pv.amount() - context.base_value.amount();

        Ok(theta)
    }
}
```

## Finite Difference Utilities

The `finite_difference` module provides standard bump sizes and helper functions:

```rust
use crate::metrics::{bump_sizes, bump_scalar_price, bump_discount_curve_parallel};

// Standard bump sizes
let spot_bump = bump_sizes::SPOT;              // 1% (0.01)
let vol_bump = bump_sizes::VOLATILITY;         // 1% (0.01)
let rate_bump = bump_sizes::INTEREST_RATE_BP;   // 1bp (in bp units: 1.0)
let spread_bump = bump_sizes::CREDIT_SPREAD_BP; // 1bp (in bp units: 1.0)

// Helper functions
let bumped_market = bump_scalar_price(&context.curves, "AAPL", 0.01)?;
let bumped_market = bump_discount_curve_parallel(&context.curves, &curve_id, 1.0)?;
```

## Best Practices

### 1. Type Safety

Always use strong typing and avoid runtime downcasting when possible:

```rust
// Good: Use trait bounds
impl<I: Instrument + CurveDependencies> MetricCalculator for MyCalc<I> {
    // ...
}

// Avoid: Runtime downcasting
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    // Only when absolutely necessary
    let bond: &Bond = context.instrument_as()?;
    // ...
}
```

### 2. Error Handling

Provide clear error messages:

```rust
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    let ytm = context.computed.get(&MetricId::Ytm)
        .ok_or_else(|| Error::Validation(
            "MyMetric requires YTM to be computed first".to_string()
        ))?;

    // ...
}
```

### 3. Determinism

Ensure calculations are deterministic:

```rust
// For Monte Carlo pricing in finite differences
instrument.pricing_overrides_mut().mc_seed_scenario = Some("delta_up".to_string());
let pv_up = instrument.value(&bumped_market, as_of)?;

instrument.pricing_overrides_mut().mc_seed_scenario = Some("delta_down".to_string());
let pv_down = instrument.value(&bumped_market, as_of)?;
```

### 4. Performance

Cache intermediate results to avoid redundant calculations:

```rust
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    // Check if already computed
    if let Some(&cached) = context.computed.get(&MetricId::MyMetric) {
        return Ok(cached);
    }

    // Compute expensive calculation once
    let cashflows = context.cashflows.get_or_insert_with(|| {
        generate_cashflows(&context.instrument, context.as_of)
    });

    // Use cached cashflows
    // ...
}
```

### 5. Documentation

Follow the documentation standards:

- Document all public types, traits, and functions
- Include working examples in doc comments
- Add mathematical formulas for complex metrics
- Reference industry standards where applicable

## Testing Strategy

### Unit Tests

Test individual calculators in isolation:

```rust
#[test]
fn test_theta_calculator() {
    let calculator = ThetaCalculator::daily();
    let mut context = create_test_context();

    let theta = calculator.calculate(&mut context).unwrap();

    assert!((theta - expected_theta).abs() < TOLERANCE);
}
```

### Integration Tests

Test metrics within the full registry:

```rust
#[test]
fn test_bond_metrics_integration() {
    let registry = standard_registry();
    let bond = create_test_bond();
    let market = create_test_market();
    let as_of = test_date();

    let base_value = bond.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        base_value,
        MetricContext::default_config(),
    );

    let metrics = vec![MetricId::Ytm, MetricId::DurationMod, MetricId::Convexity];
    let results = registry.compute(&metrics, &mut context).unwrap();

    assert!(results.contains_key(&MetricId::Ytm));
    assert!(results.contains_key(&MetricId::DurationMod));
}
```

### Property Tests

Test invariants and mathematical properties:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_dv01_sign_convention(coupon_rate in 0.01..0.10) {
        let bond = create_bond_with_coupon(coupon_rate);
        let dv01 = compute_dv01(&bond);

        // DV01 should be negative for bonds (lose value when rates rise)
        prop_assert!(dv01 < 0.0);
    }
}
```

## See Also

- **`METRICS.md`**: Comprehensive documentation of all metrics including formulas, conventions, and examples
- **`core/traits.rs`**: Core trait definitions and interfaces
- **`core/registry.rs`**: Registry implementation and dependency resolution
- **Documentation standards**: `.cursor/rules/rust/documentation.mdc`

## Contributing

When adding new metrics:

1. Follow the step-by-step guide above
2. Add comprehensive tests
3. Update `METRICS.md` with metric documentation
4. Ensure all lints pass: `make lint`
5. Ensure all tests pass: `make test-rust`
6. Add examples showing realistic usage

For questions or discussions, refer to the main project documentation or consult the development team.
