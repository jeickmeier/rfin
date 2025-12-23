# FX Barrier Option

## Features
- Up/down, in/out FX barrier options with optional rebate and Gobet–Miri adjustment toggle.
- Separate domestic/foreign discount curves, FX spot ID, and FX vol surface; supports call/put on the FX rate.
- Analytical Reiner–Rubinstein pricing with optional GBM Monte Carlo when the `mc` feature is enabled.

## Methodology & References
- Reiner & Rubinstein (1991) closed-form formulas adapted to FX (Garman–Kohlhagen carry).
- Optional Gobet–Miri (2001) barrier shift for discrete monitoring.
- Monte Carlo fallback uses the shared path-dependent engine under GBM assumptions.

## Usage Example
```rust
use finstack_valuations::instruments::fx_barrier_option::FxBarrierOption;

let option = FxBarrierOption::example();
let pv = option.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Assumes log-normal FX dynamics with deterministic domestic/foreign rates.
- Monte Carlo path requires the `mc` feature; no stochastic volatility or jumps.
- European-style payoff only; no early exercise or windowed monitoring beyond the supplied schedule.

## Pricing Methodology
- Reiner–Rubinstein continuous barrier formulas adapted to Garman–Kohlhagen FX carry; optional Gobet–Miri shift for discrete monitoring.
- Monte Carlo GBM fallback for complex cases; domestic/foreign discount curves supply carry, vol from FX surface.
- Spot FX from market data or override; rebate handled at expiry.

## Metrics
- PV plus Greeks (delta/gamma/vega/theta/rho) analytically; MC bump-and-revalue when enabled.
- Barrier hit probability proxies via MC path statistics; carry sensitivity via domestic/foreign curve bumps.
- Scenario PVs for barrier shifts and vol surface tweaks.

## Future Enhancements
- Add discrete-monitoring corrections and barrier smoothing techniques for FX calendar specifics.
- Support stochastic/local vol and jumps; quanto adjustments for cross-currency settlements.
- Include early-exercise/windowed barrier styles if demanded by products.
