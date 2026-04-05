# Quick Start — Rust

Price a fixed-rate bond and inspect its risk metrics in under 5 minutes.

## Cargo.toml

```toml
[dependencies]
finstack = { version = "0.1", features = ["core", "valuations"] }
```

## Full Example

```rust,no_run
use finstack::core::currency::Currency;
use finstack::core::money::Money;
use finstack::core::market_data::term_structures::DiscountCurve;
use finstack::core::market_data::context::MarketContext;
use finstack::valuations::instruments::bond::Bond;
use finstack::valuations::pricer::standard_registry;
use time::macros::date;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let as_of = date!(2024-01-15);
    let usd = Currency::from_str("USD")?;

    // 1. Build a discount curve
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(&[
            (0.0,  1.0),
            (1.0,  0.9524),
            (2.0,  0.9070),
            (5.0,  0.7835),
            (10.0, 0.6139),
        ])
        .build()?;

    let mut market = MarketContext::new();
    market.insert_discount(curve);

    // 2. Build a 5-year fixed-rate bond
    let bond = Bond::fixed(
        "US-TREASURY-5Y",
        Money::new(1_000_000.0, usd),
        0.045,                      // 4.5% coupon
        date!(2024-01-15),          // issue
        date!(2029-01-15),          // maturity
        "USD-OIS",                  // discount curve
    )?;

    // 3. Price with the standard registry
    let registry = standard_registry();
    let result = registry.price_with_metrics(
        &bond,
        &market,
        as_of,
        &["dirty_price", "clean_price", "ytm", "dv01", "duration_mod"],
    )?;

    println!("NPV:          {}", result.value);
    println!("Dirty Price:  {:.4}", result.metric("dirty_price").unwrap());
    println!("YTM:          {:.4}%", result.metric("ytm").unwrap() * 100.0);
    println!("DV01:         {:.2}", result.metric("dv01").unwrap());
    println!("Modified Dur: {:.4}", result.metric("duration_mod").unwrap());

    Ok(())
}
```

## Key Differences from Python

| Concept | Rust | Python |
|---------|------|--------|
| Error handling | `Result<T, E>` with `?` | Exceptions (raises `FinstackError`) |
| Currency | `Currency::from_str("USD")?` | `Currency("USD")` |
| Money | `Money::new(1_000_000.0, usd)` | `Money(1_000_000, "USD")` |
| Date literals | `date!(2024-01-15)` | `date(2024, 1, 15)` |
| Builder chain | `.build()?` returns `Result` | `.build()` raises on error |
| Metric access | `result.metric("dv01").unwrap()` | `result.metric("dv01")` |

## Next Steps

- [Architecture overview](../architecture/README.md) — crate structure and feature flags
- [Cookbooks](../cookbooks/README.md) — step-by-step recipes for common workflows
