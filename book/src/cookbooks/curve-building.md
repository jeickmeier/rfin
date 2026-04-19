# Curve Building

This cookbook covers constructing discount, forward, and hazard curves from
market data.

## 1. Discount Curve from OIS Rates

```python
from finstack.core.market_data.term_structures import DiscountCurve
from datetime import date
import math

as_of = date(2025, 1, 15)

ois_rates = {
    0.25: 0.0432,  # 3M
    0.5:  0.0428,  # 6M
    1.0:  0.0420,  # 1Y
    2.0:  0.0405,  # 2Y
    5.0:  0.0380,  # 5Y
    10.0: 0.0370,  # 10Y
    30.0: 0.0365,  # 30Y
}

knots = [(t, math.exp(-r * t)) for t, r in ois_rates.items()]
knots.insert(0, (0.0, 1.0))

disc = DiscountCurve("USD-OIS", as_of, knots)

for t in [1, 2, 5, 10]:
    fwd = disc.forward(t, t + 1)
    print(f"{t}Y-{t+1}Y forward: {fwd:.4%}")
```

## 2. Forward Curve

```python
from finstack.core.market_data.term_structures import ForwardCurve

fwd = ForwardCurve("USD-SOFR-3M", as_of, 0.25, [
    (0.0, 0.0432), (1.0, 0.0415), (2.0, 0.0400),
    (5.0, 0.0380), (10.0, 0.0370),
])

for t in [0.25, 0.5, 1.0, 2.0, 5.0]:
    print(f"3M SOFR at {t}Y: {fwd.rate(t):.4%}")
```

## 3. Multi-Curve Framework

```python
from finstack.valuations.market_context import MarketContext

ois = DiscountCurve("USD-OIS", as_of, ois_knots)
sofr_3m = ForwardCurve("USD-SOFR-3M", as_of, 0.25, sofr_knots)
sofr_1m = ForwardCurve("USD-SOFR-1M", as_of, 1/12, sofr_1m_knots)

market = MarketContext() \
    .add_discount_curve(ois) \
    .add_forward_curve(sofr_3m) \
    .add_forward_curve(sofr_1m)
```

## 4. Cross-Currency Setup

```python
from finstack.core.market_data import FxMatrix
from finstack.core.currency import Currency

eur_ois = DiscountCurve("EUR-OIS", as_of, eur_knots)

fx = FxMatrix()
fx.set_quote(Currency("EUR"), Currency("USD"), 1.10)

market = MarketContext() \
    .add_discount_curve(ois) \
    .add_discount_curve(eur_ois) \
    .add_fx_provider(fx)
```
