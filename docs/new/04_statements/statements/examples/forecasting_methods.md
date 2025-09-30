# Example: Forecasting Methods

This example demonstrates all available forecast methods.

---

## 1. Forward Fill

Carry the last known value into forecast periods.

```rust
.value("revenue", &[
    (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
    (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
])
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::ForwardFill,
    params: indexmap! {},
})
```

**Result:**
- Q1: 10,000,000 (actual)
- Q2: 11,000,000 (actual)
- Q3: 11,000,000 (forward fill from Q2)
- Q4: 11,000,000 (forward fill from Q2)

---

## 2. Growth Percentage

Apply compound growth rate.

```rust
.value("revenue", &[
    (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
])
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::GrowthPct,
    params: indexmap! {
        "rate".into() => json!(0.05),  // 5% per period
    },
})
```

**Result:**
- Q1: 10,000,000 (actual)
- Q2: 10,500,000 (10M * 1.05)
- Q3: 11,025,000 (10.5M * 1.05)
- Q4: 11,576,250 (11.025M * 1.05)

---

## 3. Normal Distribution

Sample from normal distribution (deterministic with seed).

```rust
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::Normal,
    params: indexmap! {
        "mean".into() => json!(100_000.0),
        "std_dev".into() => json!(15_000.0),
        "seed".into() => json!(42),
    },
})
```

**Use case:** Monte Carlo simulations with fixed seed for reproducibility.

---

## 4. Log-Normal Distribution

Sample from log-normal distribution (always positive).

```rust
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::LogNormal,
    params: indexmap! {
        "mean".into() => json!(100_000.0),
        "std_dev".into() => json!(15_000.0),
        "seed".into() => json!(42),
    },
})
```

**Use case:** Revenue, prices, or other strictly positive variables.

---

## 5. Override

Explicit period-by-period overrides.

```rust
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::Override,
    params: indexmap! {
        "overrides".into() => json!({
            "2025Q3": 12_000_000.0,
            "2025Q4": 13_000_000.0,
        }),
    },
})
```

**Use case:** Manual adjustments for specific periods.

---

## 6. Seasonal Pattern

Apply seasonal multipliers.

```rust
.forecast("revenue", ForecastSpec {
    method: ForecastMethod::Seasonal,
    params: indexmap! {
        "pattern".into() => json!([1.0, 1.1, 1.2, 0.9]),  // Q1-Q4 multipliers
        "base_value".into() => json!(10_000_000.0),
    },
})
```

**Result:**
- Q1: 10,000,000 (base * 1.0)
- Q2: 11,000,000 (base * 1.1)
- Q3: 12,000,000 (base * 1.2)
- Q4: 9,000,000 (base * 0.9)

---

## Combined Example

```rust
use finstack_statements::prelude::*;

fn main() -> Result<()> {
    let model = ModelBuilder::new("Forecast Comparison")
        .periods("2025Q1..2025Q4", Some("2025Q1..Q2"))?
        
        // Method 1: Forward fill
        .value("ff_revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("ff_revenue", ForecastSpec {
            method: ForecastMethod::ForwardFill,
            params: indexmap! {},
        })
        
        // Method 2: Growth
        .value("growth_revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("growth_revenue", ForecastSpec {
            method: ForecastMethod::GrowthPct,
            params: indexmap! { "rate".into() => json!(0.05) },
        })
        
        // Method 3: Seasonal
        .value("seasonal_revenue", &[
            (PeriodId::quarter(2025, 1), AmountOrScalar::Scalar(10_000_000.0)),
            (PeriodId::quarter(2025, 2), AmountOrScalar::Scalar(11_000_000.0)),
        ])
        .forecast("seasonal_revenue", ForecastSpec {
            method: ForecastMethod::Seasonal,
            params: indexmap! {
                "pattern".into() => json!([1.0, 1.1, 1.2, 0.9]),
                "base_value".into() => json!(11_000_000.0),
            },
        })
        
        .build()?;
    
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false)?;
    
    // Compare methods
    println!("Forecast Method Comparison:");
    println!("Period | Forward Fill | Growth 5% | Seasonal");
    println!("-------|--------------|-----------|----------");
    
    for period in &model.periods {
        let ff = results.nodes["ff_revenue"][&period.id];
        let growth = results.nodes["growth_revenue"][&period.id];
        let seasonal = results.nodes["seasonal_revenue"][&period.id];
        
        println!("{:6} | {:12.0} | {:9.0} | {:8.0}", 
            period.id, ff, growth, seasonal);
    }
    
    Ok(())
}
```

---

## References

- [API Reference](../API_REFERENCE.md#12-forecast-types)
- [Implementation Plan](../IMPLEMENTATION_PLAN.md#phase-4-forecasting-week-4-5)
