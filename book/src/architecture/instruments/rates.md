# Rates Instruments

## Interest Rate Swap (IRS)

Plain vanilla fixed-vs-floating interest rate swap:

**Python**

```python
from finstack.valuations.instruments import InterestRateSwap
from finstack.core.money import Money
from datetime import date

swap = InterestRateSwap.builder("IRS_USD_5Y") \
    .money(Money(10_000_000, "USD")) \
    .side("receive_fixed") \
    .fixed_rate(0.0425) \
    .start(date(2024, 1, 5)) \
    .maturity(date(2029, 1, 5)) \
    .disc_id("USD-OIS") \
    .fwd_id("USD-SOFR-3M") \
    .build()
```

**Key metrics:** `par_rate`, `pv01`, `pv_fixed`, `pv_float`, `annuity`, `dv01`,
`bucketed_dv01`

## Basis Swap

Floating-vs-floating swap on different indices (e.g., 3M SOFR vs 1M SOFR):

```python
from finstack.valuations.instruments import BasisSwap

basis = BasisSwap.builder("BASIS_3M1M") \
    .money(Money(25_000_000, "USD")) \
    .fwd_id_pay("USD-SOFR-1M") \
    .fwd_id_receive("USD-SOFR-3M") \
    .spread(0.0005) \
    .disc_id("USD-OIS") \
    .start(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .build()
```

## Cross-Currency Swap (XCCY)

Exchanges cashflows in two currencies with notional exchange:

```python
from finstack.valuations.instruments import XccySwap

xccy = XccySwap.builder("XCCY_EURUSD") \
    .domestic_notional(Money(10_000_000, "USD")) \
    .foreign_notional(Money(9_200_000, "EUR")) \
    .domestic_disc_id("USD-OIS") \
    .foreign_disc_id("EUR-OIS") \
    .start(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .build()
```

## Caps and Floors

Interest rate caps (call) and floors (put) on floating rates:

```python
from finstack.valuations.instruments import InterestRateOption

cap = InterestRateOption.builder("CAP_3Y_4PCT") \
    .money(Money(10_000_000, "USD")) \
    .option_type("cap") \
    .strike(0.04) \
    .fwd_id("USD-SOFR-3M") \
    .disc_id("USD-OIS") \
    .vol_surface("USD-CAPVOL") \
    .start(date(2024, 3, 1)) \
    .maturity(date(2027, 3, 1)) \
    .build()
```

## Swaption

Option to enter an interest rate swap:

```python
from finstack.valuations.instruments import Swaption

swaption = Swaption.builder("SWPTN_1Yx5Y") \
    .money(Money(10_000_000, "USD")) \
    .option_type("payer") \
    .strike(0.04) \
    .expiry(date(2025, 1, 15)) \
    .swap_maturity(date(2030, 1, 15)) \
    .disc_id("USD-OIS") \
    .fwd_id("USD-SOFR-3M") \
    .vol_surface("USD-SWAPTION-VOL") \
    .build()
```

**Key metrics:** `delta`, `gamma`, `vega`, `theta`

## Other Rates Instruments

| Type | Description |
|------|-------------|
| `BermudanSwaption` | Swaption with multiple exercise dates |
| `Deposit` | Money market deposit / term deposit |
| `ForwardRateAgreement` | FRA on a floating rate index |
| `InterestRateFuture` | Exchange-traded rate future |
| `IrFutureOption` | Option on a rate future |
| `CmsSwap` | Constant maturity swap |
| `CmsOption` | Cap/floor on CMS rates |
| `RangeAccrual` | Accrues coupon when rate stays in range |
| `Repo` | Repurchase agreement |
