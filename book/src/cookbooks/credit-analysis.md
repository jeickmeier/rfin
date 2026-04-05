# Credit Analysis

This cookbook covers CDS pricing, hazard curve construction, and structured
credit analysis.

## 1. Build a Hazard Curve

```python
from finstack.core.market_data.term_structures import HazardCurve, DiscountCurve
from datetime import date

as_of = date(2025, 1, 15)

disc = DiscountCurve("USD-OIS", as_of, [
    (0.0, 1.0), (1.0, 0.9524), (3.0, 0.8638),
    (5.0, 0.7835), (10.0, 0.6139),
])

hazard = HazardCurve("ACME-HZD", as_of, [
    (0.5, 0.008), (1.0, 0.010), (3.0, 0.012),
    (5.0, 0.015), (10.0, 0.020),
], recovery_rate=0.40)

for t in [1, 3, 5, 10]:
    print(f"{t}Y survival: {hazard.sp(t):.4%}")
```

## 2. Price a CDS

```python
from finstack.valuations.instruments import CreditDefaultSwap
from finstack.core.money import Money

cds = CreditDefaultSwap.buy_protection(
    "CDS-ACME-5Y",
    Money(10_000_000, "USD"),
    spread_bp=200.0,
    start_date=date(2025, 3, 20),
    maturity=date(2030, 3, 20),
    discount_curve="USD-OIS",
    credit_curve="ACME-HZD",
)

result = registry.price_with_metrics(
    cds, "isda_standard", market, as_of,
    metrics=[
        "par_spread", "risky_pv01",
        "protection_leg_pv", "premium_leg_pv",
        "cs01", "jump_to_default",
    ],
)

print(f"NPV:              {result.npv}")
print(f"Par Spread:       {result.get('par_spread'):.1f} bp")
print(f"Risky PV01:       {result.get('risky_pv01'):.4f}")
print(f"CS01:             {result.get('cs01::ACME-HZD'):.2f}")
```

## 3. CDX Index

```python
from finstack.valuations.instruments import CDSIndex

cdx = CDSIndex.builder("CDX-IG-5Y") \
    .money(Money(10_000_000, "USD")) \
    .spread_bp(60.0) \
    .maturity(date(2030, 6, 20)) \
    .disc_id("USD-OIS") \
    .credit_curve("CDX-IG-HZD") \
    .build()
```

## 4. Risky Bond Z-Spread

For bonds without a hazard curve, CS01 uses the z-spread bump method:

```python
bond = Bond.builder("CORP-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.055) \
    .disc_id("USD-OIS") \
    .build()  # no hazard_id

result = registry.price_with_metrics(
    bond, "discounting", market, as_of,
    metrics=["z_spread", "cs01"],
)

# CS01 key uses instrument ID for z-spread bonds
print(f"Z-Spread: {result.get('z_spread'):.0f} bp")
print(f"CS01:     {result.get('cs01::CORP-5Y'):.2f}")
```
