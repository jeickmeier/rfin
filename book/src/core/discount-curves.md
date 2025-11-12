# Discount Curves

Discount curves represent the time value of money, mapping future dates to present values. They are the fundamental building block for pricing all fixed income securities and derivatives.

## Market-Standard Defaults

As of the latest release, `DiscountCurve::builder()` uses **market-standard defaults**:

- **Interpolation**: `MonotoneConvex` (Hagan-West) — smooth, no-arbitrage
- **Extrapolation**: `FlatForward` — maintains positive forwards beyond last pillar
- **Validation**: Monotonic discount factors enforced by default

These defaults ensure no-arbitrage conditions and stable tail behavior consistent with institutional practice.

## Example

```rust
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::dates::Date;
use time::Month;

let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
    .build()
    .unwrap();

// Defaults: MonotoneConvex + FlatForward extrapolation
assert!(curve.df(3.0) < 1.0);
assert!(curve.forward(10.0, 15.0) > 0.0); // Stable tail forwards
```

## Overriding Defaults

You can override interpolation and extrapolation as needed:

```rust
use finstack_core::math::interp::{InterpStyle, ExtrapolationPolicy};

let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (5.0, 0.90)])
    .set_interp(InterpStyle::LogLinear)
    .extrapolation(ExtrapolationPolicy::FlatZero)
    .build()
    .unwrap();
```

## DF→FWD Conversion

Converting discount curves to forward curves **no longer clamps** forward rates or substitutes fallback values:

- Negative forwards are preserved (critical for EUR/JPY negative rate regimes)
- Malformed data (non-positive DFs, degenerate segments) returns `InputError`
- Use `ForwardCurve::builder().with_min_forward_rate(...)` for explicit floors

```rust
// Preserves negative forwards
let fwd_curve = discount_curve.to_forward_curve("EUR-FWD", 0.25)?;
```

For more details, see the [Term Structures](./term-structures.md) chapter.
