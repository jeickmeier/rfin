# Quick Start — Python

Price a fixed-rate bond and inspect its risk metrics in under 5 minutes.

## 1. Set Up Market Data

Every pricing begins with a `MarketContext` containing discount and forward
curves:

```python
from datetime import date
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve

as_of = date(2024, 1, 15)

# Build a discount curve from knot points (time in years, discount factor)
curve = DiscountCurve("USD-OIS", as_of, [
    (0.0,  1.0),
    (1.0,  0.9524),
    (2.0,  0.9070),
    (5.0,  0.7835),
    (10.0, 0.6139),
])

market = MarketContext()
market.insert(curve)
```

## 2. Build an Instrument

Instruments use a builder pattern. Here's a 5-year fixed-rate bond:

```python
from finstack.core.money import Money
from finstack.valuations.instruments import Bond

bond = Bond.builder("US-TREASURY-5Y") \
    .money(Money(1_000_000, "USD")) \
    .coupon_rate(0.045) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .build()
```

## 3. Price It

The pricer registry dispatches to the appropriate pricing model:

```python
from finstack.valuations.pricer import standard_registry

registry = standard_registry()
result = registry.price_with_metrics(
    bond,
    "discounting",
    market,
    as_of,
    metrics=["dirty_price", "clean_price", "ytm", "dv01", "duration_mod"],
)

print(f"NPV:            {result.value}")
print(f"Dirty Price:    {result.metric('dirty_price'):.4f}")
print(f"Clean Price:    {result.metric('clean_price'):.4f}")
print(f"YTM:            {result.metric('ytm'):.4%}")
print(f"DV01:           {result.metric('dv01'):.2f}")
print(f"Modified Dur:   {result.metric('duration_mod'):.4f}")
```

## 4. Add Credit Risk

Attach a hazard curve to compute credit-adjusted metrics:

```python
from finstack.core.market_data.term_structures import HazardCurve

hazard = HazardCurve("CORP-HZD", as_of, [
    (1.0, 0.01),
    (5.0, 0.015),
    (10.0, 0.02),
], recovery_rate=0.40)
market.insert(hazard)

# Rebuild the bond with a credit curve
bond_credit = Bond.builder("CORP-BOND-5Y") \
    .money(Money(1_000_000, "USD")) \
    .coupon_rate(0.055) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .hazard_id("CORP-HZD") \
    .build()

result = registry.price_with_metrics(
    bond_credit, "discounting", market, as_of,
    metrics=["dv01", "cs01", "z_spread"],
)
print(f"CS01:     {result.metric('cs01'):.2f}")
print(f"Z-Spread: {result.metric('z_spread'):.1f} bp")
```

## Next Steps

- [Curve Building cookbook](../cookbooks/curve-building.md) — bootstrap curves from market quotes
- [Swap Pricing cookbook](../cookbooks/swap-pricing.md) — IRS, basis swaps, cross-currency
- [Portfolio Valuation cookbook](../cookbooks/portfolio-valuation.md) — build and value a portfolio
- [Architecture overview](../architecture/README.md) — understand the full crate structure
