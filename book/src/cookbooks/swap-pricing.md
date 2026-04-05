# Swap Pricing

This cookbook covers pricing interest rate swaps, basis swaps, and
cross-currency swaps.

## 1. Vanilla IRS

```python
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.instruments import InterestRateSwap
from finstack.core.money import Money
from datetime import date

as_of = date(2025, 1, 15)

disc = DiscountCurve("USD-OIS", as_of, [
    (0.0, 1.0), (1.0, 0.9524), (2.0, 0.9070),
    (5.0, 0.7835), (10.0, 0.6139),
])

fwd = ForwardCurve("USD-SOFR-3M", as_of, 0.25, [
    (0.0, 0.0430), (1.0, 0.0410), (2.0, 0.0390),
    (5.0, 0.0375), (10.0, 0.0370),
])

swap = InterestRateSwap.builder("IRS_5Y") \
    .money(Money(50_000_000, "USD")) \
    .side("receive_fixed") \
    .fixed_rate(0.0400) \
    .start(date(2025, 1, 17)) \
    .maturity(date(2030, 1, 17)) \
    .disc_id("USD-OIS") \
    .fwd_id("USD-SOFR-3M") \
    .build()

result = registry.price_with_metrics(
    swap, "discounting", market, as_of,
    metrics=["par_rate", "pv01", "dv01", "bucketed_dv01"],
)

print(f"NPV:      {result.npv}")
print(f"Par Rate: {result.get('par_rate'):.4%}")
print(f"PV01:     {result.get('pv01'):.2f}")
print(f"DV01:     {result.get('dv01'):.2f}")
```

## 2. Basis Swap

```python
from finstack.valuations.instruments import BasisSwap

basis = BasisSwap.builder("BASIS_3M1M_5Y") \
    .money(Money(100_000_000, "USD")) \
    .fwd_id_pay("USD-SOFR-1M") \
    .fwd_id_receive("USD-SOFR-3M") \
    .spread(0.0005) \
    .disc_id("USD-OIS") \
    .start(date(2025, 1, 17)) \
    .maturity(date(2030, 1, 17)) \
    .build()
```

## 3. Cross-Currency Swap

```python
from finstack.valuations.instruments import XccySwap

xccy = XccySwap.builder("XCCY_EURUSD_5Y") \
    .domestic_notional(Money(50_000_000, "USD")) \
    .foreign_notional(Money(46_000_000, "EUR")) \
    .domestic_disc_id("USD-OIS") \
    .foreign_disc_id("EUR-OIS") \
    .start(date(2025, 3, 1)) \
    .maturity(date(2030, 3, 1)) \
    .build()
```

## 4. Swap Ladder

```python
for tenor in [1, 2, 3, 5, 7, 10, 15, 20, 30]:
    swap = InterestRateSwap.builder(f"IRS_{tenor}Y") \
        .money(Money(10_000_000, "USD")) \
        .side("receive_fixed") \
        .fixed_rate(0.0400) \
        .start(as_of) \
        .maturity_years(tenor) \
        .disc_id("USD-OIS") \
        .fwd_id("USD-SOFR-3M") \
        .build()

    result = registry.price_with_metrics(
        swap, "discounting", market, as_of,
        metrics=["par_rate", "dv01"],
    )
    print(f"{tenor}Y: Par={result.get('par_rate'):.4%}, "
          f"DV01={result.get('dv01'):.0f}")
```
