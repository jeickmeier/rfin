# Results Module

## Overview

The `results/` module provides standardized result envelopes for valuation operations. It ensures consistent output structure across all instrument pricing, risk calculations, and portfolio aggregations, with built-in metadata stamping for auditability and explainability.

## Architecture

```
results/
├── mod.rs                  // Re-exports and module declarations
├── valuation_result.rs     // Core result envelope (ValuationResult)
├── dataframe.rs            // DataFrame export helpers (ValuationRow)
└── README.md               // This file
```

## Core Types

### `ValuationResult`

The primary output structure returned by all pricing operations. Contains:

- **Present Value (`value`)**: The instrument's NPV as a `Money` type with currency
- **Measures (`measures`)**: Risk metrics as key-value pairs (DV01, Greeks, yields, etc.)
- **Metadata (`meta`)**: Calculation context (rounding policy, numeric mode, FX policy, timing)
- **Covenants (`covenants`)**: Optional covenant compliance results for structured products
- **Explanation (`explanation`)**: Optional computation trace for debugging and auditability

### `ValuationRow`

A flat, row-oriented representation of `ValuationResult` for DataFrame exports. Promotes common measures (DV01, convexity, duration, YTM) to top-level columns for easy analytics.

### `ResultsMeta`

Re-exported from `finstack_core::config`, this structure stamps results with:

- Numeric mode (Decimal vs f64)
- Rounding context and precision
- FX policy for cross-currency calculations
- Calculation timestamp and duration
- Parallel execution flag

## Feature Set

### 1. **Standardized Output Structure**

All pricing operations return `ValuationResult`, ensuring consistency across:

- Fixed income instruments (bonds, swaps, FRNs)
- Derivatives (options, futures, forwards)
- Structured products (ABS, MBS, CDOs, CLOs)
- Alternative assets (real estate, private credit)

### 2. **Currency Safety**

Present value is always returned as a `Money` type, which:

- Encodes currency along with the amount
- Prevents accidental cross-currency arithmetic
- Enables explicit FX conversions with policy stamping

### 3. **Flexible Metrics**

Risk measures are stored as key-value pairs in `measures`:

- Fixed income: DV01, convexity, duration, yield-to-maturity
- Options: Delta, Gamma, Vega, Theta, Rho
- Credit: CS01, spread duration, credit DV01
- Custom metrics can be added without changing the structure

### 4. **Metadata Stamping**

Every result is stamped with:

- **Numeric mode**: Decimal (deterministic) vs f64 (performance)
- **Rounding policy**: Scale, mode (half-up, half-even, etc.)
- **FX policy**: Conversion strategy for cross-currency calculations
- **Timing**: Calculation timestamp and duration
- **Parallel flag**: Whether calculation used parallel execution

This enables:

- Reproducible calculations (Decimal mode)
- Auditability (policy transparency)
- Performance comparison (timing)
- Regression testing (golden test stability)

### 5. **Covenant Integration**

For structured products with covenants (loans, ABS, MBS):

- Covenant compliance results are attached to the result
- Helper methods check if all covenants passed
- Failed covenants can be extracted for reporting

### 6. **Explainability**

Optional computation traces provide:

- Step-by-step calculation logs
- Intermediate values and data flow
- Debugging information for complex instruments
- Audit trails for regulatory compliance

### 7. **DataFrame Export**

Convert results to flat rows for analytics:

- `to_row()`: Single result to flat row
- `to_rows()`: Batch-compatible interface
- `results_to_rows()`: Batch converter
- Serde support for JSON/CSV/Parquet export

## Usage Examples

### Basic Pricing

```rust
use finstack_valuations::results::ValuationResult;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;
let pv = Money::new(1_000_000.0, Currency::USD);

let result = ValuationResult::stamped("BOND-001", as_of, pv);

println!("PV: {}", result.value.amount());
println!("Currency: {}", result.value.currency());
```

### Pricing with Metrics

```rust
use finstack_valuations::results::ValuationResult;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use indexmap::IndexMap;
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;
let pv = Money::new(1_000_000.0, Currency::USD);

// Compute risk metrics
let mut measures = IndexMap::new();
measures.insert("ytm".to_string(), 0.0475);
measures.insert("modified_duration".to_string(), 4.25);
measures.insert("dv01".to_string(), 425.0);
measures.insert("convexity".to_string(), 18.5);

let result = ValuationResult::stamped("BOND-001", as_of, pv)
    .with_measures(measures);

// Access metrics
if let Some(dv01) = result.measures.get("dv01") {
    println!("DV01: {}", dv01);
}
```

### Pricing with Covenants

```rust
use finstack_valuations::results::ValuationResult;
use finstack_valuations::covenants::CovenantReport;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;
let pv = Money::new(5_000_000.0, Currency::USD);

// Create covenant reports
let dscr_covenant = CovenantReport {
    covenant_type: "dscr".to_string(),
    passed: true,
    actual_value: Some(1.5),
    threshold: Some(1.25),
    details: Some("DSCR: 1.50x >= 1.25x threshold".to_string()),
};

let ltv_covenant = CovenantReport {
    covenant_type: "ltv".to_string(),
    passed: true,
    actual_value: Some(0.70),
    threshold: Some(0.80),
    details: Some("LTV: 70% <= 80% threshold".to_string()),
};

let result = ValuationResult::stamped("LOAN-001", as_of, pv)
    .with_covenant("dscr_test", dscr_covenant)
    .with_covenant("ltv_test", ltv_covenant);

// Check covenant compliance
if result.all_covenants_passed() {
    println!("All covenants passed");
} else {
    let failed = result.failed_covenants();
    println!("Failed covenants: {:?}", failed);
}
```

### Custom Metadata Stamping

```rust
use finstack_valuations::results::ValuationResult;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use finstack_core::config::{FinstackConfig, results_meta};
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;
let pv = Money::new(1_000_000.0, Currency::USD);

// Pre-construct metadata once for batch pricing (performance optimization)
let config = FinstackConfig::default();
let meta = results_meta(&config);

let result = ValuationResult::stamped_with_meta("BOND-001", as_of, pv, meta);
```

### Batch Pricing with DataFrame Export

```rust
use finstack_valuations::results::{ValuationResult, results_to_rows};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;

// Price multiple instruments
let results = vec![
    ValuationResult::stamped("BOND-001", as_of, Money::new(1_000_000.0, Currency::USD)),
    ValuationResult::stamped("BOND-002", as_of, Money::new(500_000.0, Currency::EUR)),
    ValuationResult::stamped("BOND-003", as_of, Money::new(750_000.0, Currency::GBP)),
];

// Convert to rows for DataFrame construction
let rows = results_to_rows(&results);

// Export to JSON/CSV/Parquet
let json = serde_json::to_string(&rows)?;
```

### Portfolio Integration

```rust
use finstack_valuations::results::ValuationResult;
use finstack_valuations::metrics::MetricId;

// In portfolio valuation context
fn value_single_position(
    position: &Position,
    market: &MarketContext,
    as_of: Date,
    metrics: &[MetricId],
) -> Result<PositionValue> {
    // Price instrument with metrics
    let valuation_result = position
        .instrument
        .price_with_metrics(market, as_of, metrics)?;

    let value_native = valuation_result.value;

    // Scale by quantity
    let scaled_value = value_native * position.quantity;

    // Convert to base currency with FX
    let value_base = convert_to_base_currency(scaled_value, market.fx)?;

    Ok(PositionValue {
        position_id: position.id.clone(),
        value_native,
        value_base,
        valuation_result: Some(valuation_result),
    })
}
```

### Explainability

```rust
use finstack_valuations::results::ValuationResult;
use finstack_core::explain::ExplanationTrace;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use time::Month;

let as_of = create_date(2025, Month::January, 15)?;
let pv = Money::new(1_000_000.0, Currency::USD);

// Create explanation trace during pricing
let mut trace = ExplanationTrace::new("bond_pricing");
// ... add trace entries during calculation ...

let result = ValuationResult::stamped("BOND-001", as_of, pv)
    .with_explanation(trace);

// Access explanation for debugging
if let Some(explanation) = &result.explanation {
    println!("Computation trace: {:#?}", explanation);
}
```

## How Results Flow Through the System

### 1. **Instrument Pricing**

All instrument pricers return `ValuationResult`:

```rust
// In bond pricer
pub fn price_bond(
    bond: &Bond,
    market: &MarketContext,
    as_of: Date,
) -> Result<ValuationResult> {
    let pv = calculate_present_value(bond, market, as_of)?;
    Ok(ValuationResult::stamped(&bond.id, as_of, pv))
}
```

### 2. **Metrics Calculation**

Metrics are computed and attached:

```rust
// In metrics calculator
pub fn calculate_dv01(
    instrument: &impl Instrument,
    market: &MarketContext,
    as_of: Date,
) -> Result<ValuationResult> {
    let base_result = instrument.value(market, as_of)?;
    let shifted_result = instrument.value(&shifted_market, as_of)?;

    let dv01 = (shifted_result.amount() - base_result.amount()) / 0.0001;

    let mut measures = IndexMap::new();
    measures.insert("dv01".to_string(), dv01);

    Ok(ValuationResult::stamped(&instrument.id(), as_of, base_result)
        .with_measures(measures))
}
```

### 3. **Portfolio Aggregation**

Results are collected and aggregated:

```rust
// In portfolio valuation
let position_values: Vec<PositionValue> = portfolio
    .positions
    .iter()
    .map(|position| {
        let valuation_result = position.instrument.price_with_metrics(market, as_of, metrics)?;
        Ok(PositionValue {
            position_id: position.id.clone(),
            value_native: valuation_result.value,
            value_base: convert_with_fx(valuation_result.value, market.fx)?,
            valuation_result: Some(valuation_result),
        })
    })
    .collect::<Result<_>>()?;
```

### 4. **Export and Reporting**

Results are converted to DataFrames for analysis:

```rust
// Export to analytics
let rows = results_to_rows(&valuation_results);
let df = polars::DataFrame::from_rows(&rows)?;
df.write_parquet("valuations.parquet")?;
```

## Design Principles

### 1. **Separation of Concerns**

- **Value**: Always in `value` field as `Money` type (currency-safe)
- **Metrics**: Derived risk measures in `measures` map (extensible)
- **Metadata**: Calculation context in `meta` (auditability)
- **Covenants**: Optional compliance in `covenants` (structured products)
- **Explanation**: Optional trace in `explanation` (debugging)

This separation enables:

- Clean interfaces (value always available, metrics optional)
- Extensibility (add new metrics without changing structure)
- Performance (skip unused features)

### 2. **Builder Pattern**

Results are constructed incrementally:

```rust
ValuationResult::stamped(id, as_of, pv)
    .with_measures(measures)
    .with_covenants(covenants)
    .with_explanation(trace)
```

This provides:

- Flexibility (add only what you need)
- Clarity (explicit intent)
- Type safety (compile-time checking)

### 3. **Metadata Stamping**

Every result carries metadata about:

- **How** it was calculated (numeric mode, rounding)
- **When** it was calculated (timestamp, duration)
- **What policies** were applied (FX conversion strategy)

This enables:

- Reproducibility (re-run with same policies)
- Auditability (trace calculation provenance)
- Regression testing (detect calculation drift)

### 4. **Consistency Across Instruments**

All instruments return the same result structure:

- Vanilla instruments (bonds, loans): PV + basic metrics
- Derivatives (options): PV + Greeks
- Structured products (ABS, MBS): PV + metrics + covenants
- Alternative assets: PV + custom metrics

This ensures:

- Uniform portfolio aggregation
- Consistent reporting interfaces
- Easy instrument addition

## Adding New Features

### Adding a New Metric

To add a new risk metric to results:

1. **Compute the metric** in your instrument pricer or metrics calculator:

```rust
// In your metrics calculator
let new_metric_value = calculate_new_metric(instrument, market)?;
```

2. **Add to measures map**:

```rust
measures.insert("new_metric".to_string(), new_metric_value);
```

3. **(Optional) Add to ValuationRow** if it should be a top-level column:

```rust
// In dataframe.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationRow {
    // ... existing fields ...

    /// Your new metric (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_metric: Option<f64>,
}

// Update to_row() method
impl ValuationResult {
    pub fn to_row(&self) -> ValuationRow {
        ValuationRow {
            // ... existing fields ...
            new_metric: self.measures.get("new_metric").copied(),
        }
    }
}
```

4. **Use consistent naming** via `MetricId` enum (see `metrics/mod.rs`):

```rust
measures.insert(MetricId::NewMetric.as_str(), new_metric_value);
```

### Adding a New Result Field

To add a new top-level field to `ValuationResult`:

1. **Add the field** to `ValuationResult` struct in `valuation_result.rs`:

```rust
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValuationResult {
    // ... existing fields ...

    /// Your new field with documentation
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub new_field: Option<YourType>,
}
```

2. **Update constructors** to initialize the field:

```rust
impl ValuationResult {
    pub fn stamped_with_meta(
        instrument_id: &str,
        as_of: Date,
        value: Money,
        meta: ResultsMeta,
    ) -> Self {
        Self {
            // ... existing fields ...
            new_field: None,
        }
    }
}
```

3. **Add a builder method**:

```rust
impl ValuationResult {
    /// Attach your new field.
    pub fn with_new_field(mut self, data: YourType) -> Self {
        self.new_field = Some(data);
        self
    }
}
```

4. **Update documentation** with usage examples

5. **Add tests**:

```rust
#[test]
fn test_new_field() {
    let result = ValuationResult::stamped("TEST", as_of, pv)
        .with_new_field(your_data);

    assert!(result.new_field.is_some());
}
```

### Adding a New Export Format

To add a new export format (e.g., custom DataFrame schema):

1. **Create a new struct** in `dataframe.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRow {
    pub custom_field_1: String,
    pub custom_field_2: f64,
    // ... your custom schema ...
}
```

2. **Implement conversion**:

```rust
impl ValuationResult {
    pub fn to_custom_row(&self) -> CustomRow {
        CustomRow {
            custom_field_1: self.instrument_id.clone(),
            custom_field_2: self.value.amount(),
            // ... map your fields ...
        }
    }
}
```

3. **Add batch converter**:

```rust
pub fn results_to_custom_rows(results: &[ValuationResult]) -> Vec<CustomRow> {
    results.iter().map(|r| r.to_custom_row()).collect()
}
```

## Performance Considerations

### Metadata Stamping

For batch pricing, pre-construct metadata to avoid repeated allocation:

```rust
// GOOD: Construct metadata once
let meta = results_meta(&config);
for instrument in instruments {
    let result = ValuationResult::stamped_with_meta(&instrument.id, as_of, pv, meta.clone());
}

// BAD: Reconstruct metadata for each result
for instrument in instruments {
    let result = ValuationResult::stamped(&instrument.id, as_of, pv);  // Allocates new config
}
```

### Measures Map

Use `IndexMap` for stable iteration order (required for determinism):

```rust
use indexmap::IndexMap;

let mut measures = IndexMap::new();
measures.insert("dv01".to_string(), dv01);
```

### Parallel Aggregation

When aggregating results in parallel, use deterministic reduction:

```rust
use rayon::prelude::*;

let results: Vec<ValuationResult> = instruments
    .par_iter()
    .map(|instrument| price_instrument(instrument, market))
    .collect::<Result<_>>()?;

// Deterministic aggregation (serial reduction)
let total_value = results.iter()
    .try_fold(Money::zero(base_ccy), |acc, r| {
        acc.checked_add(r.value)
    })?;
```

## Testing

### Unit Tests

Test result construction and accessors:

```rust
#[test]
fn test_result_with_measures() {
    let mut measures = IndexMap::new();
    measures.insert("dv01".to_string(), 1000.0);

    let result = ValuationResult::stamped("TEST", as_of, pv)
        .with_measures(measures);

    assert_eq!(result.measures.get("dv01"), Some(&1000.0));
}
```

### Golden Tests

Serialize results for regression testing:

```rust
#[test]
fn test_result_serialization() {
    let result = create_test_result();
    let json = serde_json::to_string(&result).unwrap();

    // Compare against golden file
    assert_eq!(json, include_str!("golden/result.json"));
}
```

### Property Tests

Test invariants:

```rust
#[test]
fn test_covenant_consistency() {
    let result = create_result_with_covenants();

    if result.all_covenants_passed() {
        assert!(result.failed_covenants().is_empty());
    }
}
```

## Related Modules

- **`finstack_core::config::ResultsMeta`**: Metadata stamping infrastructure
- **`finstack_core::explain`**: Explainability framework
- **`finstack_valuations::metrics`**: Risk metrics definitions (`MetricId`)
- **`finstack_valuations::covenants`**: Covenant compliance reporting
- **`finstack_portfolio::results`**: Portfolio-level result aggregation

## References

- [Valuations Crate README](../../README.md)
- [Core Config Documentation](../../../../core/src/config/)
- [Explainability Module](../../../../core/src/explain/)
- [Portfolio Results](../../../../portfolio/src/results.rs)
