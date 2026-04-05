# Equity Instruments

## Equity Option

European or American equity options with Black-Scholes or tree-based pricing:

**Python**

```python
from finstack.valuations.instruments import EquityOption
from finstack.core.money import Money
from datetime import date

call = EquityOption.builder("ACME-CALL-150") \
    .ticker("ACME") \
    .strike(150.0) \
    .expiry(date(2024, 12, 31)) \
    .notional(Money(100.0, "USD")) \
    .option_type("call") \
    .exercise_style("european") \
    .disc_id("USD-OIS") \
    .spot_id("EQUITY-SPOT") \
    .vol_surface("EQUITY-VOL") \
    .div_yield_id("EQUITY-DIVYIELD") \
    .build()

result = registry.price_with_metrics(
    call, "black_scholes", market, as_of,
    metrics=["delta", "gamma", "vega", "theta", "rho"],
)
```

**Key metrics (Greeks):**

| Metric | Definition |
|--------|------------|
| `delta` | $\partial V / \partial S$ — sensitivity to spot |
| `gamma` | $\partial^2 V / \partial S^2$ — convexity in spot |
| `vega` | $\partial V / \partial \sigma$ — sensitivity to implied vol |
| `theta` | $\partial V / \partial t$ — time decay |
| `rho` | $\partial V / \partial r$ — sensitivity to rates |

## Variance Swap

Swap of realized vs implied variance:

```python
from finstack.valuations.instruments import VarianceSwap

varswap = VarianceSwap.builder("VARSWAP-ACME-1Y") \
    .notional_vega(Money(100_000, "USD")) \
    .strike_vol(0.25) \
    .start(date(2024, 1, 15)) \
    .maturity(date(2025, 1, 15)) \
    .disc_id("USD-OIS") \
    .spot_id("EQUITY-SPOT") \
    .build()
```

## Other Equity Types

| Type | Description |
|------|-------------|
| `Equity` | Cash equity position |
| `EquityTotalReturnSwap` | TRS on equity underlier |
| `EquityIndexFuture` | Exchange-traded index future |
| `PrivateMarketsFund` | Private equity/credit fund position |
| `RealEstateAsset` | Direct real estate holding |
| `LeveredRealEstateEquity` | Levered real estate equity position |
