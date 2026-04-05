# FX Instruments

## FX Forward

Agreement to exchange currencies at a future date:

**Python**

```python
from finstack.valuations.instruments import FxForward
from finstack.core.money import Money
from datetime import date

fwd = FxForward.builder("EURUSD-3M") \
    .base_currency("EUR") \
    .quote_currency("USD") \
    .notional(Money(5_000_000, "EUR")) \
    .forward_rate(1.0920) \
    .settlement(date(2024, 4, 15)) \
    .domestic_disc_id("USD-OIS") \
    .foreign_disc_id("EUR-OIS") \
    .build()
```

## FX Swap

Simultaneous near and far leg FX transactions:

```python
from finstack.valuations.instruments import FxSwap

fx_swap = FxSwap.builder("EURUSD-SWAP") \
    .base_currency("EUR") \
    .quote_currency("USD") \
    .notional(Money(5_000_000, "EUR")) \
    .near_date(date(2024, 1, 17)) \
    .far_date(date(2024, 7, 17)) \
    .near_rate(1.0865) \
    .far_rate(1.0920) \
    .domestic_discount_curve("USD-OIS") \
    .foreign_discount_curve("EUR-OIS") \
    .build()
```

## FX Option

Vanilla FX options (European):

```python
from finstack.valuations.instruments import FxOption

fx_call = FxOption.builder("EURUSD-CALL-1.10") \
    .base_currency("EUR") \
    .quote_currency("USD") \
    .notional(Money(10_000_000, "EUR")) \
    .strike(1.10) \
    .expiry(date(2024, 7, 15)) \
    .option_type("call") \
    .disc_id("USD-OIS") \
    .vol_surface("EURUSD-VOL") \
    .build()
```

## Other FX Types

| Type | Description |
|------|-------------|
| `FxSpot` | Spot FX position |
| `FxBarrierOption` | Knock-in/knock-out FX options |
| `FxDigitalOption` | Binary payout FX options |
| `FxTouchOption` | One-touch / no-touch FX options |
| `FxVarianceSwap` | FX variance swap |
| `XccySwap` | Cross-currency interest rate swap |
