# Volatility Surfaces

Volatility surfaces provide implied volatility as a function of strike and
expiry. They are required for option pricing (equity, FX, rates).

## VolSurface

A 2D implied volatility grid (strike × maturity):

**Rust**

```rust,no_run
use finstack_core::market_data::surfaces::VolSurface;

let surface = VolSurface::builder("EQ-VOL")
    .expiries(&[0.25, 0.5, 1.0])       // 3M, 6M, 1Y
    .strikes(&[90.0, 100.0, 110.0])    // strike grid
    .row(&[0.22, 0.18, 0.20])          // 3M vols
    .row(&[0.21, 0.17, 0.19])          // 6M vols
    .row(&[0.20, 0.16, 0.18])          // 1Y vols
    .build()?;
```

**Python**

```python
from finstack.core.market_data.surfaces import VolSurface

surface = VolSurface.builder("EQ-VOL") \
    .expiries([0.25, 0.5, 1.0]) \
    .strikes([90.0, 100.0, 110.0]) \
    .row([0.22, 0.18, 0.20]) \
    .row([0.21, 0.17, 0.19]) \
    .row([0.20, 0.16, 0.18]) \
    .build()
```

## Key Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `value_checked(expiry, strike)` | `f64` | Vol with boundary validation |
| `value_clamped(expiry, strike)` | `f64` | Vol clamped to grid bounds |
| `value_extrapolated(expiry, strike, fwd)` | `f64` | Out-of-grid extrapolation |
| `bump_point(expiry, strike, pct)` | `VolSurface` | Single-point bump for vega |
| `apply_bucket_bump(bumps)` | `VolSurface` | Multi-point scenario bumping |
| `grid_shape()` | `(usize, usize)` | Grid dimensions |

## FX Delta Vol Surface

FX vol surfaces use delta-space quoting conventions:

- **ATM**: At-the-money volatility
- **25Δ RR** (Risk Reversal): Call vol − Put vol (skew)
- **25Δ BF** (Butterfly): (Call vol + Put vol)/2 − ATM (wing premium)

Recovery formulas:

$$\sigma_{\text{call}} = \text{ATM} + BF + 0.5 \times RR$$
$$\sigma_{\text{put}} = \text{ATM} + BF - 0.5 \times RR$$

## Strike Representations

The `secondary_axis` builder option controls how strikes are interpreted:

| Axis | Description |
|------|-------------|
| `Strike` | Absolute strike price |
| `Premium` | Premium-adjusted strike |
| `Delta` | Delta-space (for FX) |

## SABR Model

The SABR stochastic volatility model can be used to generate smooth vol smiles:

```python
surface = VolSurface.from_sabr(
    alpha=0.04,     # initial vol
    beta=0.5,       # CEV exponent
    rho=-0.3,       # vol-spot correlation
    nu=0.4,         # vol of vol
    forward=100.0,
    expiry=1.0,
    strikes=[85, 90, 95, 100, 105, 110, 115],
)
```

SABR is the standard model for swaption and cap/floor volatility interpolation.
