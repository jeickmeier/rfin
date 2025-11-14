# P&L Attribution

Multi-period P&L attribution decomposes daily MTM changes into constituent factors to explain "Why did my position's value change from T₀ to T₁?"

## Overview

P&L attribution systematically breaks down total P&L into:
- **Carry**: Time decay (theta) and accruals
- **Rates Curves**: Discount and forward curve shifts (IR risk)
- **Credit Curves**: Hazard curve shifts (credit spread risk)
- **Inflation Curves**: Inflation curve shifts
- **Correlations**: Base correlation curve changes (structured credit)
- **FX**: Foreign exchange rate changes
- **Volatility**: Implied volatility changes
- **Model Parameters**: Prepayment, default, recovery rates, etc.
- **Market Scalars**: Dividends, equity/commodity prices, inflation indices
- **Residual**: Unexplained P&L (cross-effects, non-linearities)

## Attribution Methodologies

### Parallel Attribution

Independent factor isolation. Each factor is analyzed separately by restoring T₀ values for that factor while keeping all others at T₁.

**Advantages:**
- Clear factor independence
- Intuitive interpretation

**Disadvantages:**
- Factors don't sum exactly to total (residual captures cross-effects)
- More repricing required

```rust
use finstack_valuations::attribution::attribute_pnl_parallel;

let attribution = attribute_pnl_parallel(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &config,
)?;

println!("Total P&L: {}", attribution.total_pnl);
println!("Carry: {}", attribution.carry);
println!("Rates: {}", attribution.rates_curves_pnl);
println!("Residual: {} ({:.2}%)", 
    attribution.residual,
    attribution.meta.residual_pct
);
```

### Waterfall Attribution

Sequential factor application. Factors are applied one-by-one in a specified order, with each factor's P&L computed after applying all previous factors.

**Advantages:**
- Factors sum to total P&L (minimal residual by construction)
- Suitable for risk reporting

**Disadvantages:**
- Order matters (different orders → different factor P&Ls)
- Less intuitive than parallel

```rust
use finstack_valuations::attribution::{
    attribute_pnl_waterfall, default_waterfall_order
};

let factor_order = default_waterfall_order();
// Or customize: vec![Carry, RatesCurves, CreditCurves, Fx]

let attribution = attribute_pnl_waterfall(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &config,
    factor_order,
)?;

// Residual should be minimal (< 0.01%)
assert!(attribution.residual_within_tolerance(0.01, 1.0));
```

### Metrics-Based Attribution

Fast linear approximation using pre-computed risk metrics (Theta, DV01, CS01, etc.).

**Advantages:**
- Very fast (no repricing)
- Uses existing ValuationResults

**Disadvantages:**
- Linear approximation only (ignores convexity)
- Less accurate for large market moves
- Larger residuals

```rust
use finstack_valuations::attribution::attribute_pnl_metrics_based;

// Requires pre-computed valuations with metrics
let metrics = vec![MetricId::Theta, MetricId::Dv01, MetricId::Cs01];
let val_t0 = instrument.price_with_metrics(&market_t0, as_of_t0, &metrics)?;
let val_t1 = instrument.price_with_metrics(&market_t1, as_of_t1, &metrics)?;

let attribution = attribute_pnl_metrics_based(
    &instrument,
    &market_t0,
    &market_t1,
    &val_t0,
    &val_t1,
    as_of_t0,
    as_of_t1,
)?;
```

### DV01, RiskyPv01, and CS01: Key Differences

For credit instruments (CDS, CDS Index, CDS Option, CDS Tranche), three distinct risk metrics measure different sensitivities:

**DV01 (Dollar Value of 1 Basis Point)**:
- Measures sensitivity to interest rate (discount curve) changes
- Computed as: `DV01 = PV(discount_rate + 1bp) − PV(base)`
- Applies parallel +1bp bump to the discount curve used for present value calculations
- Sign depends on instrument structure (can be positive or negative)

**RiskyPv01** (Credit-Specific):
- Measures sensitivity to the running premium/coupon spread
- Represents the present value of a 1bp premium stream, survival-weighted
- Computed using risky annuity calculation from the CDS pricer
- Always positive for standard protection buyer/seller relationships

**CS01 (Credit Spread 01)**:
- Measures sensitivity to credit spread (hazard curve) changes
- Computed by bumping the underlying credit/hazard curve by +1bp
- Captures default probability and loss-given-default impacts
- Sign depends on whether buying or selling protection

These metrics are complementary and capture different aspects of credit instrument risk.

## Factor Definitions

### Carry
Time decay (theta) and accruals between T₀ and T₁. Computed by pricing at T₁ date with T₀ market (frozen).

### Rates Curves
Impact of discount and forward curve shifts on PV. Isolates IR risk by restoring T₀ discount/forward curves.

### Credit Curves
Impact of hazard curve shifts on PV. Relevant for CDS,CDS Index, CDS Option, CDS Tranche, corporate bonds, term loans, revolving credit, structured credit.

### Inflation Curves
Impact of inflation curve shifts on PV. Relevant for inflation-linked bonds and swaps.

### Correlations
Impact of base correlation curve changes. Relevant for structured credit (CDO tranches).

### FX
Impact of foreign exchange rate changes. Isolates FX risk by restoring T₀ FX matrix.

### Volatility
Impact of implied volatility changes. Relevant for options and volatility-sensitive instruments.

### Model Parameters
Impact of model-specific parameter changes:
- Prepayment speeds (for MBS/ABS)
- Default rates (for structured credit)
- Recovery rates (for credit instruments)
- Conversion ratios (for convertible bonds)

### Market Scalars
Impact of changes in dividends, equity prices, commodity prices, inflation indices.

## FX Attribution Semantics

### Internal FX Exposure vs Translation

The FX attribution factor captures two distinct effects:

1. **Internal FX Exposure (Pricing-Side)**:
   - How FX rate changes affect the instrument's value in its native currency
   - Relevant for cross-currency swaps, quanto options, FX-linked instruments
   - Default behavior when using instrument currency

2. **FX Translation (Reporting-Side)**:
   - How FX rate changes affect conversion to a base/reporting currency
   - Relevant when aggregating multi-currency portfolios
   - Requires explicit base currency parameter (future enhancement)

**Current Implementation**: The FX factor isolates internal FX exposure effects. For single-currency instruments (e.g., USD bond), FX P&L is correctly near-zero since there's no pricing dependency on FX rates.

**Example**: A USD corporate bond has:
- FX P&L ≈ 0 (no internal FX exposure)
- If reported in EUR, translation effect would be separate (not currently captured)

For cross-currency instruments (e.g., EUR/USD cross-currency swap):
- FX P&L captures how EUR/USD rate changes affect the swap's USD value
- Both legs' present values depend on the FX rate for fair value calculation

### Currency and Units

All P&L values are returned in the **instrument's native currency** by default. For portfolio aggregation, use `portfolio::attribution::attribute_portfolio_pnl()` which converts to a common base currency.

Exported CSVs include explicit `currency` columns to prevent unit ambiguity.

## Attribution Metadata

Each attribution result includes comprehensive metadata in `AttributionMeta`:

```rust
pub struct AttributionMeta {
    pub method: AttributionMethod,          // Parallel, Waterfall, or MetricsBased
    pub t0: Date,                          // Start date
    pub t1: Date,                          // End date
    pub instrument_id: String,             // Instrument identifier
    pub num_repricings: usize,             // Count of repricing operations
    pub tolerance_abs: f64,                // Absolute tolerance threshold
    pub tolerance_pct: f64,                // Percentage tolerance threshold
    pub residual_pct: f64,                 // Actual residual as percentage
    pub rounding: RoundingContext,         // Rounding policy applied
    pub fx_policy: Option<FxPolicyMeta>,   // FX conversion policy (if applied)
    pub notes: Vec<String>,                // Diagnostic notes/warnings
}
```

**Rounding Context**: Stamps the numeric mode (Decimal/f64), rounding mode, and scale policies used during computation. This ensures deterministic reproducibility.

**FX Policy**: Records the FX conversion strategy (CashflowDate, PeriodEnd, etc.) and target currency when FX conversions are applied. Enables full audit trails for cross-currency calculations.

**Diagnostic Notes**: Warnings for:
- Model parameter extraction/modification failures
- Skipped factors (instrument doesn't support)
- Currency validation issues
- Missing market data

**Tolerance Thresholds**: Separate absolute and percentage tolerances provide flexibility:
- `tolerance_abs`: Dollar/unit threshold (e.g., $1.00)
- `tolerance_pct`: Percentage of total P&L (e.g., 0.01%)
- Actual check uses the larger of the two

Use `residual_within_meta_tolerance()` to check against stored thresholds, or `residual_within_tolerance(pct, abs)` for custom thresholds.

## Portfolio Attribution

Aggregate attribution across all positions:

```rust
use finstack_portfolio::attribution::{
    attribute_portfolio_pnl, AttributionMethod
};

let attribution = attribute_portfolio_pnl(
    &portfolio,
    &market_t0,
    &market_t1,
    &config,
    AttributionMethod::Parallel,
)?;

println!("Portfolio P&L: {}", attribution.total_pnl);
println!("Total Carry: {}", attribution.carry);

// Position-by-position breakdown
for (position_id, pos_attr) in &attribution.by_position {
    println!("{}: {}", position_id, pos_attr.total_pnl);
}
```

## Explainability

Use the `explain()` method to generate a structured tree:

```rust
println!("{}", attribution.explain());
```

Output:
```
Total P&L: $125,430
  ├─ Carry: $45,000 (35.8%)
  ├─ Rates Curves: $65,000 (51.7%)
  ├─ Credit Curves: $5,000 (4.0%)
  ├─ FX: $12,000 (9.5%)
  ├─ Vol: $2,000 (1.6%)
  └─ Residual: -$1,570 (-1.2%)
```

## Data Export

Export attribution to CSV for analysis:

```rust
// Summary CSV
let csv = attribution.to_csv();
std::fs::write("attribution.csv", csv)?;

// Detailed curve attribution
if let Some(csv) = attribution.rates_detail_to_csv() {
    std::fs::write("rates_detail.csv", csv)?;
}
```

## Residual Analysis

Check if residual is within acceptable tolerance:

```rust
// 0.1% or $100, whichever is larger
let is_acceptable = attribution.residual_within_tolerance(0.1, 100.0);

if !is_acceptable {
    println!("Warning: Residual {} ({:.2}%) exceeds tolerance",
        attribution.residual,
        attribution.meta.residual_pct
    );
}
```

Large residuals may indicate:
- Missing market factors
- Non-linear effects (convexity, cross-gamma)
- Model limitations
- Pricing inconsistencies

## Performance Considerations

- **Parallel attribution**: 6-10 repricings per instrument (one per factor)
- **Waterfall attribution**: N+2 repricings (N = number of factors)
- **Metrics-based**: 0 repricings (uses existing metrics)

For large portfolios, consider:
- Using metrics-based for real-time dashboards
- Running parallel/waterfall overnight for detailed reports
- Caching intermediate results

## Model Parameters Attribution

### Supported Instruments

**Structured Credit (ABS, RMBS, CMBS, CLO)**:
- Prepayment speeds (CPR, constant and time-varying)
- Default rates (CDR, constant and time-varying)
- Recovery rates (constant severity, time-varying)

```rust
// Attribution will automatically detect parameter changes
let attribution = attribute_pnl_parallel(&rmbs, &market_t0, &market_t1, ...)?;

// Model params P&L captures prepayment/default/recovery changes
println!("Model Params P&L: {}", attribution.model_params_pnl);
```

**Convertible Bonds**:
- Conversion ratios
- Conversion policies

```rust
// Conversion ratio changes are automatically attributed
let attribution = attribute_pnl_parallel(&convertible, &market_t0, &market_t1, ...)?;
println!("Conversion P&L: {}", attribution.model_params_pnl);
```

### Parameter Shift Measurement

The following shifts are automatically measured:
- **PSA Multiplier**: Converted to equivalent CPR basis points (0.1 PSA = 60bp)
- **CPR/CDR**: Direct basis point comparison
- **Recovery/Severity**: Percentage point comparison
- **Conversion Ratio**: Percentage change

## Market Scalars Attribution

### Supported Scalars

- **Equity Prices**: Spot price changes for equity instruments
- **Dividends**: Dividend schedule changes for equity options
- **Inflation Indices**: CPI/RPI changes for inflation-linked bonds
- **Commodity Prices**: Price changes for commodity-linked instruments

```rust
// Create markets with different equity prices
let market_t0 = MarketContext::new()
    .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));

let market_t1 = MarketContext::new()
    .insert_price("AAPL", MarketScalar::Price(Money::new(185.0, Currency::USD)));

let attribution = attribute_pnl_parallel(&equity, &market_t0, &market_t1, ...)?;

// Market scalars P&L captures the price change
println!("Scalars P&L: {}", attribution.market_scalars_pnl);
```

## Complete Example

```rust
use finstack_valuations::attribution::attribute_pnl_parallel;
use finstack_valuations::instruments::structured_credit::StructuredCredit;

// Create RMBS at T₀ with PSA 100%
let rmbs_t0 = create_rmbs_with_psa(1.0);

// Market changes:
// - Rates increased by 50bp
// - Spreads tightened by 10bp  
// - PSA speeds increased to 150%

let attribution = attribute_pnl_parallel(
    &rmbs_t0,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &config,
)?;

// See full breakdown
println!("{}", attribution.explain());

// Output:
// Total P&L: $-50,000
//   ├─ Carry: $5,000 (theta)
//   ├─ Rates Curves: $-60,000 (rates up, price down)
//   ├─ Credit Curves: $15,000 (spreads tightened)
//   ├─ Model Params: $-10,000 (faster prepayments reduce WAL)
//   └─ Residual: $0 (0.0%)
```

## JSON Serialization

Attribution supports full JSON serialization for external integration via versioned envelopes.

### Request Envelope

```json
{
  "schema": "finstack.attribution/1",
  "attribution": {
    "instrument": {
      "type": "bond",
      "spec": { /* Bond specification */ }
    },
    "market_t0": { /* Market snapshot at T₀ */ },
    "market_t1": { /* Market snapshot at T₁ */ },
    "as_of_t0": "2025-01-15",
    "as_of_t1": "2025-01-16",
    "method": "Parallel",
    "config": {
      "tolerance_abs": 0.01,
      "tolerance_pct": 0.001
    }
  }
}
```

### Rust API

```rust
use finstack_valuations::attribution::AttributionEnvelope;

// Load from JSON
let envelope = AttributionEnvelope::from_json(json_str)?;

// Execute
let result = envelope.execute()?;

// Serialize result
let result_json = result.to_string()?;
```

### Python API

```python
from finstack.valuations import attribute_pnl_from_json, attribution_result_to_json
import json

# Load and parse JSON request
with open("attribution_request.json") as f:
    spec_json = f.read()

# Execute attribution
attribution = attribute_pnl_from_json(spec_json)

# Print results
print(attribution.explain())

# Serialize result
result_json = attribution_result_to_json(attribution)
with open("attribution_result.json", "w") as f:
    f.write(result_json)
```

### When to Use JSON vs Programmatic API

**Use JSON envelopes when:**
- Integrating with external systems via REST APIs
- Storing attribution requests for audit/replay
- Building attribution pipelines from configuration files
- Need stable wire formats across language boundaries

**Use programmatic API when:**
- Working within Rust or Python directly
- Building interactive applications
- Performance is critical (avoid JSON parse overhead)
- Working with in-memory market data

## Limitations

### Current Limitations

1. **Per-Tenor Attribution**: Not yet implemented for detailed curve analysis (planned)
2. **Multi-day Batch**: Single T₀→T₁ attribution only (batch support planned)
3. **WASM JSON API**: Envelope support not yet implemented for WebAssembly

### Future Enhancements

- Bucketed attribution (per-tenor for curves)
- Multi-day batch attribution
- Parallel execution with Rayon for portfolio attribution
- Enhanced model parameter extraction for exotic derivatives
- WASM JSON envelope support

