# Market Data

The market data layer provides containers for discount curves, forward curves,
hazard curves, volatility surfaces, and FX rates. All market data is assembled
into a `MarketContext` that instruments query at pricing time.

## MarketContext

`MarketContext` is the central container that holds all market data for a
valuation. Instruments look up curves by their string ID at pricing time:

**Python**

```python
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve

market = MarketContext()
market.insert(discount_curve)    # Looked up by curve ID (e.g., "USD-OIS")
market.insert(forward_curve)     # Looked up by curve ID (e.g., "USD-SOFR-3M")
market.insert(hazard_curve)      # Looked up by curve ID (e.g., "ACME-HZD")

# Retrieve by ID
ois = market.get_discount("USD-OIS")
sofr = market.get_forward("USD-SOFR-3M")
```

## Term Structure Taxonomy

| Type | Purpose | Key Method | Math |
|------|---------|------------|------|
| `DiscountCurve` | Present value discounting | `df(t)` | $DF(t) = e^{-r(t) \cdot t}$ |
| `ForwardCurve` | Rate projection for floating legs | `rate(t)` | Simple forward rate |
| `HazardCurve` | Credit default modeling | `sp(t)` | $S(t) = e^{-\int_0^t \lambda(s) ds}$ |
| `InflationCurve` | CPI/breakeven expectations | `cpi(t)` | Forward CPI index |
| `PriceCurve` | Commodity/equity forward prices | `price(t)` | Forward price |
| `BaseCorrelationCurve` | CDX tranche correlation | `corr(K)` | Base correlation by attachment |

## Interpolation

All term structures support pluggable interpolation:

| Style | Description | Use Case |
|-------|-------------|----------|
| `LinearDf` | Linear in discount factors | Simple, fast |
| `LogLinearDf` | Linear in log(DF) → constant zero rates | Standard for most curves |
| `MonotoneConvex` | Hagan-West smooth, arbitrage-free | Production quality |
| `CubicHermite` | PCHIP shape-preserving | Smooth forwards |
| `PiecewiseQuadraticForward` | C² forwards in log-DF space | Research |

Extrapolation options: `FlatZero` (standard) or `FlatForward`.

## Detail Pages

- [Discount Curves](discount-curves.md) — risk-free discounting
- [Forward Curves](forward-curves.md) — rate projection
- [Hazard Curves](hazard-curves.md) — credit survival probabilities
- [Volatility Surfaces](volatility-surfaces.md) — option pricing surfaces
- [FX Rates](fx-rates.md) — cross-currency rates and triangulation
