# Options Pricing

This cookbook covers pricing equity options, caps/floors, and swaptions.

## 1. Equity Option (Black-Scholes)

```python
from finstack.valuations.instruments import EquityOption
from finstack.core.money import Money
from finstack.core.market_data.surfaces import VolSurface
from datetime import date

as_of = date(2025, 1, 15)

vol = VolSurface.builder("ACME-VOL") \
    .expiries([0.25, 0.5, 1.0]) \
    .strikes([80, 90, 100, 110, 120]) \
    .row([0.28, 0.24, 0.20, 0.22, 0.26]) \
    .row([0.27, 0.23, 0.19, 0.21, 0.25]) \
    .row([0.26, 0.22, 0.18, 0.20, 0.24]) \
    .build()

call = EquityOption.builder("ACME-CALL-100") \
    .ticker("ACME") \
    .strike(100.0) \
    .expiry(date(2026, 1, 15)) \
    .notional(Money(1000.0, "USD")) \
    .option_type("call") \
    .exercise_style("european") \
    .disc_id("USD-OIS") \
    .vol_surface("ACME-VOL") \
    .build()

result = registry.price_with_metrics(
    call, "black_scholes", market, as_of,
    metrics=["delta", "gamma", "vega", "theta", "rho"],
)

print(f"Price:  {result.npv}")
print(f"Delta:  {result.get('delta'):.4f}")
print(f"Gamma:  {result.get('gamma'):.6f}")
print(f"Vega:   {result.get('vega'):.2f}")
print(f"Theta:  {result.get('theta'):.2f}")
```

## 2. Interest Rate Cap

```python
from finstack.valuations.instruments import InterestRateOption

cap = InterestRateOption.builder("CAP_3Y_4PCT") \
    .money(Money(10_000_000, "USD")) \
    .option_type("cap") \
    .strike(0.04) \
    .fwd_id("USD-SOFR-3M") \
    .disc_id("USD-OIS") \
    .vol_surface("USD-CAPVOL") \
    .start(date(2025, 3, 1)) \
    .maturity(date(2028, 3, 1)) \
    .build()

result = registry.price_with_metrics(
    cap, "bachelier", market, as_of,
    metrics=["delta", "vega", "gamma"],
)
print(f"Cap Price: {result.npv}")
```

## 3. Swaption

```python
from finstack.valuations.instruments import Swaption

swaption = Swaption.builder("SWPTN_1Yx5Y") \
    .money(Money(10_000_000, "USD")) \
    .option_type("payer") \
    .strike(0.04) \
    .expiry(date(2026, 1, 15)) \
    .swap_maturity(date(2031, 1, 15)) \
    .disc_id("USD-OIS") \
    .fwd_id("USD-SOFR-3M") \
    .vol_surface("USD-SWAPTION-VOL") \
    .build()

result = registry.price_with_metrics(
    swaption, "black76", market, as_of,
    metrics=["delta", "vega", "gamma"],
)
print(f"Swaption Price: {result.npv}")
print(f"Vega:           {result.get('vega'):.2f}")
```
