# Discount Curves

A `DiscountCurve` maps time to discount factors: $DF(t) = e^{-r(t) \cdot t}$
where $r(t)$ is the continuously compounded zero rate. It is the most
fundamental market data object — used for present value discounting across all
instrument types.

## Construction

Curves are built from (time, discount factor) knot points using a builder:

**Rust**

```rust,no_run
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;

let curve = DiscountCurve::builder("USD-OIS")
    .base_date(date!(2025-01-15))
    .knots(&[
        (0.0,  1.0),      // today
        (0.25, 0.9988),   // 3M
        (0.5,  0.9975),   // 6M
        (1.0,  0.9524),   // 1Y
        (2.0,  0.9070),   // 2Y
        (5.0,  0.7835),   // 5Y
        (10.0, 0.6139),   // 10Y
        (30.0, 0.2314),   // 30Y
    ])
    .interp(InterpStyle::MonotoneConvex)
    .build()?;
```

**Python**

```python
from finstack.core.market_data.term_structures import DiscountCurve
from datetime import date

curve = DiscountCurve("USD-OIS", date(2025, 1, 15), [
    (0.0,  1.0),
    (1.0,  0.9524),
    (2.0,  0.9070),
    (5.0,  0.7835),
    (10.0, 0.6139),
    (30.0, 0.2314),
])
```

## Key Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `df(t)` | `f64` | Discount factor at time `t` (years) |
| `forward(t1, t2)` | `f64` | Simple forward rate between two times |
| `df_on_date(date)` | `f64` | Discount factor to a calendar date |
| `df_between_dates(d1, d2)` | `f64` | Discount between two dates |
| `forward_on_dates(d1, d2)` | `f64` | Forward rate between two dates |
| `df_batch(times)` | `Vec<f64>` | Batch evaluation (vectorized) |

## Builder Options

| Option | Default | Description |
|--------|---------|-------------|
| `interp(InterpStyle)` | `Linear` | Interpolation method |
| `extrapolation(policy)` | `FlatForward` | Behavior beyond last knot |
| `day_count(dc)` | Auto-detected from ID | Convention for date → time |
| `enforce_no_arbitrage()` | Off | DF monotonicity + forward rate floor |
| `min_forward_rate(f64)` | None | Custom forward rate floor (e.g., -50bp) |
| `allow_non_monotonic_with_floor()` | Off | Allows negative rates (EUR/JPY) with -5% safety floor |

## Arbitrage Guards

By default, curves allow any knot values. For production use,
`enforce_no_arbitrage()` validates:

1. Discount factors are monotonically decreasing
2. Implied forward rates stay above a floor (default -50bp)

For negative-rate environments (EUR, JPY), use
`allow_non_monotonic_with_floor()` with a -5% safety floor.

## Convention Auto-Detection

The curve ID drives automatic convention inference:

| ID Pattern | Day Count | Notes |
|------------|-----------|-------|
| `*-OIS` | Act/365F | Overnight index swap curves |
| `*-LIBOR*` | Act/360 | Legacy LIBOR curves |
| `*-SOFR*` | Act/360 | SOFR term curves |
| Custom | Must specify | Use `.day_count()` builder method |
