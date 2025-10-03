# Phase 7 Example: Results Export to Polars DataFrames

This example demonstrates the comprehensive DataFrame export functionality added in Phase 7 of the finstack-statements crate.

## Overview

The Phase 7 example showcases:

1. **Long Format Export** - Converting results to `(node_id, period_id, value)` format
2. **Wide Format Export** - Converting results to periods-as-rows, nodes-as-columns format
3. **Filtered Export** - Exporting only specific nodes of interest
4. **Financial Analysis** - Using exported DataFrames for analysis
5. **Format Comparison** - Understanding when to use each format

## Running the Example

```bash
cargo run --example statements_phase7_example --features polars_export
```

## Example Output

The example builds a Tech Startup P&L model with:
- 4 quarters (2025Q1-Q4)
- Revenue with 10% QoQ growth
- Cost structure (COGS, OpEx, Marketing)
- Key financial metrics (margins, EBITDA)

### Long Format Export

Best for time-series operations and database storage:

```
32 rows × 3 columns
(node_id, period_id, value)
```

### Wide Format Export

Best for human-readable reports and pivot tables:

```
4 rows × 9 columns
(period_id, revenue, cogs, opex, marketing, gross_profit, gross_margin, ebitda, ebitda_margin)
```

### Filtered Export

Export only key metrics:

```
20 rows × 3 columns
(filtered to: revenue, gross_profit, gross_margin, ebitda, ebitda_margin)
```

## Key Features Demonstrated

### 1. Long Format
- Ideal for time-series databases
- Easy to filter and group
- Compatible with pandas, Polars, and other data tools

### 2. Wide Format
- Human-readable structure
- Perfect for Excel exports
- Great for dashboards and reports

### 3. Filtered Export
- Reduce data size
- Focus on specific metrics
- Useful for executive summaries

### 4. Financial Analysis
The example includes quarter-by-quarter analysis showing:
- Revenue growth from $500K to $665.5K (33.1% total growth)
- EBITDA margin improvement from 5.0% to 13.4%
- Full P&L breakdown

## Use Cases

### Data Science & Analytics
- Export to Polars/pandas for advanced analysis
- Time-series forecasting
- Statistical modeling

### Business Intelligence
- Create executive dashboards
- Generate financial reports
- Track KPIs over time

### Data Integration
- Export to time-series databases
- Feed into BI tools (Tableau, Power BI)
- API responses in structured format

## Code Highlights

```rust
// Build model with forecasts
let model = ModelBuilder::new("Tech Startup P&L")
    .periods("2025Q1..Q4", Some("2025Q2"))?
    .value("revenue", &[...])
    .forecast("revenue", ForecastSpec {
        method: ForecastMethod::GrowthPct,
        params: indexmap! { "rate".into() => json!(0.10) },
    })
    .compute("ebitda", "gross_profit - opex - marketing")?
    .build()?;

// Evaluate
let mut evaluator = Evaluator::new();
let results = evaluator.evaluate(&model, false)?;

// Export to different formats
let df_long = results.to_polars_long()?;
let df_wide = results.to_polars_wide()?;
let df_filtered = results.to_polars_long_filtered(&["revenue", "ebitda"])?;
```

## Related Documentation

- [Phase 7 Summary](../../finstack/statements/PHASE7_SUMMARY.md)
- [API Reference](../../docs/new/04_statements/statements/API_REFERENCE.md)
- [Implementation Plan](../../docs/new/04_statements/statements/IMPLEMENTATION_PLAN.md)

## Next Steps

After understanding DataFrame export:
1. Explore Phase 6 for capital structure integration
2. Try combining with Python bindings for pandas export
3. Build custom analytics workflows with Polars

