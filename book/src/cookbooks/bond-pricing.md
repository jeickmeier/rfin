# Bond Pricing

This cookbook walks through pricing a fixed-rate corporate bond, computing
risk metrics, and analyzing spread sensitivity.

## Setup

```python
from finstack.core.money import Money
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.valuations.instruments import Bond
from finstack.valuations.pricer import standard_registry
from datetime import date

as_of = date(2025, 1, 15)
registry = standard_registry()
```

## 1. Build Market Data

```python
disc = DiscountCurve("USD-OIS", as_of, [
    (0.0,  1.0),
    (0.5,  0.9975),
    (1.0,  0.9524),
    (2.0,  0.9070),
    (5.0,  0.7835),
    (10.0, 0.6139),
])

hazard = HazardCurve("ACME-HZD", as_of, [
    (1.0, 0.010),
    (3.0, 0.012),
    (5.0, 0.015),
], recovery_rate=0.40)
```

## 2. Build the Bond

```python
bond = Bond.builder("ACME-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.045) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .hazard_id("ACME-HZD") \
    .build()
```

## 3. Price with Metrics

```python
from finstack.valuations.market_context import MarketContext

market = MarketContext() \
    .add_discount_curve(disc) \
    .add_hazard_curve(hazard)

result = registry.price_with_metrics(
    bond, "discounting", market, as_of,
    metrics=[
        "dirty_price", "clean_price", "ytm",
        "duration_mod", "convexity",
        "dv01", "cs01", "z_spread",
        "accrued",
    ],
)

print(f"NPV:            {result.npv}")
print(f"Clean Price:    {result.get('clean_price'):.4f}")
print(f"Dirty Price:    {result.get('dirty_price'):.4f}")
print(f"Accrued:        {result.get('accrued'):.2f}")
print(f"YTM:            {result.get('ytm'):.4%}")
print(f"Mod Duration:   {result.get('duration_mod'):.4f}")
print(f"Convexity:      {result.get('convexity'):.4f}")
print(f"DV01:           {result.get('dv01'):.2f}")
print(f"CS01:           {result.get('cs01::ACME-5Y'):.2f}")
print(f"Z-Spread:       {result.get('z_spread'):.1f} bp")
```

## 4. Bucketed DV01

```python
result = registry.price_with_metrics(
    bond, "discounting", market, as_of,
    metrics=["bucketed_dv01"],
)

for key, value in sorted(result.metrics.items()):
    if key.startswith("bucketed_dv01"):
        print(f"  {key}: {value:.2f}")
```

## Rust Equivalent

```rust,no_run
use finstack_valuations::instruments::Bond;
use finstack_core::money::Money;

let bond = Bond::builder("ACME-5Y")
    .money(Money::new(10_000_000.0, Currency::USD))
    .coupon_rate(0.045)
    .frequency(Frequency::Semiannual)
    .issue(date!(2024-01-15))
    .maturity(date!(2029-01-15))
    .disc_id("USD-OIS")
    .hazard_id("ACME-HZD")
    .build()?;

let result = registry.price_with_metrics(
    &bond, "discounting", &market, as_of, &metrics,
)?;
```
