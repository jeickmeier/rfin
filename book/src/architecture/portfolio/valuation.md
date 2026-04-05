# Portfolio Valuation

Portfolio valuation prices all positions in a portfolio using the pricer
registry and aggregates results.

## value_portfolio()

The primary entry point:

```python
from finstack.portfolio import value_portfolio

results = value_portfolio(
    portfolio,
    market,          # MarketContext with all curves/surfaces
    as_of,           # valuation date
    metrics=["dv01", "cs01", "bucketed_dv01"],
)

# Per-position results
for pos_id, result in results.position_results.items():
    print(f"{pos_id}: NPV={result.npv}, DV01={result.get('dv01')}")

# Aggregated portfolio-level
print(f"Total NPV: {results.total_npv}")
print(f"Total DV01: {results.aggregate('dv01')}")
```

## Multi-Currency Aggregation

When positions span multiple currencies, aggregation requires FX conversion:

```python
results = value_portfolio(
    portfolio, market, as_of,
    reporting_currency="USD",  # convert all to USD
)

# All NPVs and metrics are in USD
print(results.total_npv)  # Money(xxx, USD)
```

The `FxProvider` in the `MarketContext` supplies the required FX rates.

## Parallel Execution

In Rust, portfolio valuation uses Rayon for parallel pricing:

```rust,no_run
use finstack_portfolio::value_portfolio;

// Positions are priced in parallel across CPU cores
let results = value_portfolio(&portfolio, &market, as_of, &metrics)?;
```

This parallelism is transparent in Python — the GIL is released during
Rust execution, so all cores are utilized.

## Factor Decomposition

Decompose portfolio risk into factor contributions:

```python
from finstack.portfolio import factor_decomposition

decomp = factor_decomposition(
    portfolio, market, as_of,
    factors=["rates", "credit", "fx", "equity"],
)

for factor, contribution in decomp.items():
    print(f"{factor}: {contribution}")
```
