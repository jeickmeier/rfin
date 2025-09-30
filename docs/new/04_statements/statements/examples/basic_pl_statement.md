# Example: Basic P&L Statement Model

This example demonstrates how to build a simple profit & loss (P&L) statement model.

---

## Model Overview

**Periods:** 2025Q1 through 2025Q4  
**Actuals:** Q1-Q2  
**Forecast:** Q3-Q4

**Metrics:**
- Revenue (actuals + 5% growth forecast)
- COGS (60% of revenue)
- Operating Expenses (forward fill from actuals)
- Derived: Gross Profit, Operating Income

---

## Code

```rust
use finstack_statements::prelude::*;

fn main() -> Result<()> {
    // Build the model
    let model = ModelBuilder::new("Acme Corp P&L")
        // 1. Define periods
        .periods("2025Q1..2025Q4", Some("2025Q1..Q2"))?
        
        // 2. Revenue with actuals and forecast
        .value("revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        
        // 3. COGS as percentage of revenue
        .compute("cogs", "revenue * 0.6")?
        
        // 4. Operating expenses with forward fill
        .value("operating_expenses", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(2_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(2_100_000.0)),
        ])
        .forecast("operating_expenses", ForecastSpec {
            method: ForecastMethod::ForwardFill,
            params: indexmap! {},
        })
        
        // 5. Derived metrics
        .compute("gross_profit", "revenue - cogs")?
        .compute("operating_income", "gross_profit - operating_expenses")?
        .compute("gross_margin", "gross_profit / revenue")?
        
        .build()?;
    
    // Evaluate the model
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    // Export to DataFrame (long format)
    let df = results.to_polars_long()?;
    println!("{}", df);
    
    // Or export wide format
    let df_wide = results.to_polars_wide()?;
    println!("\nWide format:\n{}", df_wide);
    
    Ok(())
}
```

---

## Expected Output

### Long Format

```
node_id             | period_id | value
--------------------|-----------|----------------
revenue             | 2025Q1    | 10_000_000.0
revenue             | 2025Q2    | 11_000_000.0
revenue             | 2025Q3    | 11_550_000.0
revenue             | 2025Q4    | 12_127_500.0
cogs                | 2025Q1    | 6_000_000.0
cogs                | 2025Q2    | 6_600_000.0
cogs                | 2025Q3    | 6_930_000.0
cogs                | 2025Q4    | 7_276_500.0
gross_profit        | 2025Q1    | 4_000_000.0
gross_profit        | 2025Q2    | 4_400_000.0
gross_profit        | 2025Q3    | 4_620_000.0
gross_profit        | 2025Q4    | 4_851_000.0
operating_expenses  | 2025Q1    | 2_000_000.0
operating_expenses  | 2025Q2    | 2_100_000.0
operating_expenses  | 2025Q3    | 2_100_000.0
operating_expenses  | 2025Q4    | 2_100_000.0
operating_income    | 2025Q1    | 2_000_000.0
operating_income    | 2025Q2    | 2_300_000.0
operating_income    | 2025Q3    | 2_520_000.0
operating_income    | 2025Q4    | 2_751_000.0
gross_margin        | 2025Q1    | 0.40
gross_margin        | 2025Q2    | 0.40
gross_margin        | 2025Q3    | 0.40
gross_margin        | 2025Q4    | 0.40
```

### Wide Format

```
period_id | revenue      | cogs        | gross_profit | operating_expenses | operating_income | gross_margin
----------|--------------|-------------|--------------|--------------------|-----------------|--------------
2025Q1    | 10_000_000.0 | 6_000_000.0 | 4_000_000.0  | 2_000_000.0        | 2_000_000.0     | 0.40
2025Q2    | 11_000_000.0 | 6_600_000.0 | 4_400_000.0  | 2_100_000.0        | 2_300_000.0     | 0.40
2025Q3    | 11_550_000.0 | 6_930_000.0 | 4_620_000.0  | 2_100_000.0        | 2_520_000.0     | 0.40
2025Q4    | 12_127_500.0 | 7_276_500.0 | 4_851_000.0  | 2_100_000.0        | 2_751_000.0     | 0.40
```

---

## Key Concepts Demonstrated

1. **Period Definition** — Parse period range with actuals cutoff
2. **Value Nodes** — Explicit values for actuals
3. **Forecast Methods** — GrowthPct and ForwardFill
4. **Calculated Nodes** — Formulas referencing other nodes
5. **Precedence** — Actuals (value) override forecasts for Q1-Q2
6. **DataFrame Export** — Both long and wide formats

---

## Variations

### Add Interest Expense

```rust
.value("interest_expense", &[
    (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(100_000.0)),
])
.forecast("interest_expense", ForecastSpec {
    method: ForecastMethod::ForwardFill,
    params: indexmap! {},
})
.compute("ebt", "operating_income - interest_expense")?
.compute("taxes", "if(ebt > 0, ebt * 0.25, 0)")?
.compute("net_income", "ebt - taxes")?
```

### Use Built-in Metrics

```rust
.with_builtin_metrics()?  // Adds fin.gross_profit, fin.gross_margin, etc.
.add_metric("fin.gross_profit")?
.add_metric("fin.operating_income")?
```

---

## References

- [API Reference](../API_REFERENCE.md) — Full DSL syntax
- [README](../README.md) — Quick start guide
