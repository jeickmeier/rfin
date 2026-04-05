# Forward Curves

A `ForwardCurve` provides simple forward rate projections for floating-rate
indices (SOFR, EURIBOR, etc.). It is used to project future coupon rates on
floating-rate instruments.

## Construction

**Rust**

```rust,no_run
use finstack_core::market_data::term_structures::ForwardCurve;

let curve = ForwardCurve::builder("USD-SOFR3M", 0.25)  // 3-month tenor
    .base_date(date!(2025-01-15))
    .knots(&[
        (0.0,  0.0430),  // spot
        (1.0,  0.0410),  // 1Y
        (2.0,  0.0390),  // 2Y
        (5.0,  0.0375),  // 5Y
        (10.0, 0.0370),  // 10Y
    ])
    .build()?;
```

**Python**

```python
from finstack.core.market_data.term_structures import ForwardCurve
from datetime import date

curve = ForwardCurve("USD-SOFR3M", date(2025, 1, 15), 0.25, [
    (0.0,  0.0430),
    (1.0,  0.0410),
    (5.0,  0.0375),
    (10.0, 0.0370),
])
```

## Key Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `rate(t)` | `f64` | Forward rate at time `t` |
| `tenor()` | `f64` | Index tenor in years (e.g., 0.25 for 3M) |
| `reset_lag()` | `i32` | Business days from fixing to effective |

## Rate Index Conventions

Conventions are auto-detected from the curve ID:

| ID Pattern | Tenor | Reset Lag | Day Count |
|------------|-------|-----------|----------|
| `*-SOFR` | O/N | 0 | Act/360 |
| `*-SOFR3M` | 3M | 0 | Act/360 |
| `*-ESTR` | O/N | 0 | Act/360 |
| `*-EURIBOR6M` | 6M | 2 | Act/360 |
| `*-SONIA` | O/N | 0 | Act/365F |
| `*-TIBOR` | various | 2 | Act/365F |

## Multi-Curve Framework

In modern interest rate modeling, discounting and projection use separate curves:

- **Discount curve** (e.g., `USD-OIS`) — for present value discounting
- **Forward curve** (e.g., `USD-SOFR3M`) — for projecting floating rates

This separation accounts for the OIS-LIBOR basis spread and is the standard
post-2008 framework. An IRS references both:

```python
swap = InterestRateSwap.builder("IRS_5Y") \
    .disc_id("USD-OIS") \           # discounting
    .fwd_id("USD-SOFR-3M") \        # projection
    .build()
```
