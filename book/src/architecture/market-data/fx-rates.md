# FX Rates

FX rates are provided through the `FxProvider` trait and cached via `FxMatrix`.
Finstack enforces explicit FX conversion — there is no implicit currency
conversion anywhere in the library.

## FxProvider Trait

The core abstraction for FX rate supply:

```rust,no_run
pub trait FxProvider: Send + Sync {
    fn rate(&self, query: &FxQuery) -> Result<FxResult>;
}
```

## SimpleFxProvider

A simple in-memory FX provider for testing and small setups:

**Python**

```python
from finstack.core.money import SimpleFxProvider, FxMatrix
from finstack.core.currency import Currency
from datetime import date

provider = SimpleFxProvider()
provider.set_quote(Currency("EUR"), Currency("USD"), 1.10)
provider.set_quote(Currency("GBP"), Currency("USD"), 1.27)
provider.set_quote(Currency("USD"), Currency("JPY"), 155.0)

matrix = FxMatrix(provider)

# Direct quote
rate = matrix.rate(Currency("EUR"), Currency("USD"), date(2025, 1, 15))
# rate = 1.10

# Reciprocal (automatically computed)
rate_inv = matrix.rate(Currency("USD"), Currency("EUR"), date(2025, 1, 15))
# rate_inv ≈ 0.9091

# Triangulation (EUR→JPY via USD)
rate_cross = matrix.rate(Currency("EUR"), Currency("JPY"), date(2025, 1, 15))
# rate_cross ≈ 170.5 (1.10 × 155.0)
```

## FxQuery

An FX lookup is parameterized by:

| Field | Type | Description |
|-------|------|-------------|
| `from` | `Currency` | Source currency |
| `to` | `Currency` | Target currency |
| `on` | `Date` | Valuation date |
| `policy` | `FxConversionPolicy` | When to observe the rate |

### Conversion Policies

| Policy | Description |
|--------|-------------|
| `CashflowDate` | Use the rate on the cashflow date |
| `PeriodEnd` | Use the rate at period end |
| `PeriodAverage` | Average rate over the period |
| `Custom` | User-defined rate observation |

## FxMatrix Caching

The `FxMatrix` wraps an `FxProvider` with an LRU cache for O(1) repeated
lookups. It handles:

- **Reciprocals**: USD/EUR = 1 / (EUR/USD)
- **Triangulation**: EUR/JPY = EUR/USD × USD/JPY
- **Caching**: Bounded LRU cache prevents unbounded memory growth

## Cross-Currency Instruments

Cross-currency instruments (XCCY swaps, FX forwards) reference specific FX
rates through the market context rather than holding embedded rates.
