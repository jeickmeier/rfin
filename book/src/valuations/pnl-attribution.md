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

## Factor Definitions

### Carry
Time decay (theta) and accruals between T₀ and T₁. Computed by pricing at T₁ date with T₀ market (frozen).

### Rates Curves
Impact of discount and forward curve shifts on PV. Isolates IR risk by restoring T₀ discount/forward curves.

### Credit Curves
Impact of hazard curve shifts on PV. Relevant for CDS, corporate bonds, structured credit.

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

## Limitations

### Current Limitations

1. **Model Parameters**: Instrument-specific support required. Currently returns zero for most instruments.
2. **Market Scalars**: Limited by private fields in MarketContext. Will be enhanced in future releases.
3. **Per-Tenor Attribution**: Not yet implemented for detailed curve analysis.

### Future Enhancements

- Bucketed attribution (per-tenor for curves)
- Multi-day batch attribution
- Parallel execution with Rayon
- Enhanced model parameter extraction for all instrument types

