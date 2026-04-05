# Exotic Products

Exotic instruments require advanced pricing models, typically Monte Carlo
simulation or tree-based methods.

## Autocallable

Structured product with automatic early redemption when the underlying exceeds
a barrier on observation dates:

```python
from finstack.valuations.instruments import Autocallable

autocall = Autocallable.builder("AUTOCALL-SPX-1Y") \
    .notional(Money(1_000_000, "USD")) \
    .underlying("SPX") \
    .autocall_barrier(1.05) \
    .coupon_barrier(0.95) \
    .coupon_rate(0.08) \
    .ki_barrier(0.65) \
    .observation_dates([date(2024, 7, 15), date(2025, 1, 15)]) \
    .maturity(date(2025, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("SPX-VOL") \
    .build()
```

## Barrier Option

Option whose existence depends on the underlying crossing a barrier:

| Variant | Description |
|---------|-------------|
| Up-and-In | Activates when spot rises above barrier |
| Up-and-Out | Extinguishes when spot rises above barrier |
| Down-and-In | Activates when spot falls below barrier |
| Down-and-Out | Extinguishes when spot falls below barrier |

```python
from finstack.valuations.instruments import BarrierOption

barrier = BarrierOption.builder("BARRIER-DI-CALL") \
    .notional(Money(100_000, "USD")) \
    .strike(100.0) \
    .barrier(85.0) \
    .barrier_type("down_and_in") \
    .option_type("call") \
    .expiry(date(2025, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("EQ-VOL") \
    .build()
```

## Asian Option

Payoff based on the average price over a period:

```python
from finstack.valuations.instruments import AsianOption

asian = AsianOption.builder("ASIAN-AVG-CALL") \
    .notional(Money(100_000, "USD")) \
    .strike(100.0) \
    .option_type("call") \
    .averaging("arithmetic") \
    .expiry(date(2025, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("EQ-VOL") \
    .build()
```

## Other Exotic Types

| Type | Description |
|------|-------------|
| `LookbackOption` | Payoff based on max/min observed price |
| `QuantoOption` | Option with payout in a different currency |
| `CliquetOption` | Series of forward-starting options |
| `RangeAccrual` | Accrues when a rate stays within a range |
