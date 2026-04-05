# Credit Derivative Instruments

## Credit Default Swap (CDS)

A CDS provides protection against default of a reference entity:

**Python**

```python
from finstack.valuations.instruments import CreditDefaultSwap
from finstack.core.money import Money
from datetime import date, timedelta

as_of = date(2024, 1, 15)

cds = CreditDefaultSwap.buy_protection(
    "CDS-ACME-5Y",
    notional=Money(10_000_000, "USD"),
    spread_bp=200.0,                        # 200bp running spread
    start_date=as_of + timedelta(days=1),
    maturity=date(2029, 3, 20),             # IMM date
    discount_curve="USD-OIS",
    credit_curve="ACME-HZD",
)
```

**Key metrics:** `par_spread`, `risky_pv01`, `risky_annuity`,
`protection_leg_pv`, `premium_leg_pv`, `jump_to_default`, `expected_loss`,
`cs01`, `bucketed_cs01`

### Pricing Model

CDS pricing follows the ISDA Standard Model:

$$PV_{\text{protection}} = (1-R) \int_0^T DF(t) \cdot dPD(t)$$

$$PV_{\text{premium}} = s \sum_i \Delta_i \cdot DF(t_i) \cdot S(t_i)$$

where $R$ is recovery rate, $DF(t)$ is discount factor, $S(t)$ is survival
probability, and $s$ is the running spread.

## CDS Index (CDX/iTraxx)

A CDS index represents a portfolio of single-name CDS:

```python
from finstack.valuations.instruments import CDSIndex
from finstack.core.money import Money
from datetime import date

cdx = CDSIndex.builder("CDX-IG-5Y") \
    .index_name("CDX.NA.IG") \
    .series(40) \
    .version(1) \
    .money(Money(10_000_000, "USD")) \
    .fixed_coupon_bp(100.0) \
    .start_date(date(2024, 3, 20)) \
    .maturity(date(2029, 6, 20)) \
    .discount_curve("USD-OIS") \
    .credit_curve("CDX-IG-HZD") \
    .build()
```

## CDX Tranche

Synthetic CDO tranches with defined attachment/detachment points:

```python
from finstack.valuations.instruments import CDSTranche
from finstack.core.money import Money
from datetime import date

tranche = CDSTranche.builder("CDX-IG-3-7") \
    .index_name("CDX.NA.IG") \
    .series(40) \
    .notional(Money(10_000_000, "USD")) \
    .attach_pct(0.03) \
    .detach_pct(0.07) \
    .running_coupon_bp(500.0) \
    .maturity(date(2029, 6, 20)) \
    .discount_curve("USD-OIS") \
    .credit_index_curve("CDX-IG-HZD") \
    .side("buy_protection") \
    .build()
```

Tranche pricing uses the Gaussian copula model with base correlation.

## CDS Option

Option to enter a CDS at a given spread:

```python
from finstack.valuations.instruments import CDSOption
from finstack.core.money import Money
from datetime import date

cds_opt = CDSOption.builder("CDXOPT-5Y") \
    .money(Money(5_000_000, "USD")) \
    .strike_spread_bp(60.0) \
    .expiry(date(2024, 6, 20)) \
    .cds_maturity(date(2029, 6, 20)) \
    .discount_curve("USD-OIS") \
    .credit_curve("CDX-IG-HZD") \
    .vol_surface("CDX-VOL") \
    .option_type("call") \
    .build()
```
