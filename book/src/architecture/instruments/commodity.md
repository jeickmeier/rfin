# Commodity Instruments

## Commodity Forward

Agreement to buy/sell a commodity at a future date:

**Python**

```python
from finstack.valuations.instruments import CommodityForward
from finstack.core.money import Money
from datetime import date

fwd = CommodityForward.builder("WTI-DEC25") \
    .notional(1000.0) \
    .forward_price(Money(75.50, "USD")) \
    .delivery_date(date(2025, 12, 15)) \
    .price_curve("WTI-FWD") \
    .disc_id("USD-OIS") \
    .build()
```

## Commodity Swap

Fixed-vs-floating commodity price swap:

```python
from finstack.valuations.instruments import CommoditySwap

swap = CommoditySwap.builder("NATGAS-SWAP-1Y") \
    .notional(10000.0) \
    .fixed_price(Money(3.50, "USD")) \
    .price_curve("HENRY-HUB-FWD") \
    .disc_id("USD-OIS") \
    .start(date(2024, 4, 1)) \
    .maturity(date(2025, 4, 1)) \
    .build()
```

## Commodity Option

Vanilla call/put on commodity prices:

```python
from finstack.valuations.instruments import CommodityOption

option = CommodityOption.builder("WTI-CALL-80") \
    .notional(1000.0) \
    .strike(80.0) \
    .expiry(date(2024, 12, 15)) \
    .option_type("call") \
    .price_curve("WTI-FWD") \
    .vol_surface("WTI-VOL") \
    .disc_id("USD-OIS") \
    .build()
```

## Other Commodity Types

| Type | Description |
|------|-------------|
| `CommodityAsianOption` | Average price option (arithmetic/geometric) |
| `CommoditySwaption` | Option to enter a commodity swap |
| `CommoditySpreadOption` | Option on the spread between two commodities |
