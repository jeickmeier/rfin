# Range Accrual

## Features
- Coupon accrues when underlying stays within `[lower_bound, upper_bound]` across an observation schedule; optional payment date override.
- Supports quanto inputs (equity/FX correlation, FX vol surface) plus dividend yield, discount curve, and vol surface IDs.
- Monte Carlo GBM pricer for path-dependent accrual under the `mc` feature.

## Methodology & References
- Path-dependent Monte Carlo using shared engine; accrual fraction based on proportion of observations inside the range.
- Quanto adjustment uses supplied correlation and FX vol when provided; otherwise standard single-currency payoff.
- Deterministic curves/vols; no analytic approximation implemented.

## Usage Example
```rust
use finstack_valuations::instruments::rates::range_accrual::RangeAccrual;

let note = RangeAccrual::example();
let pv = note.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Requires `mc` feature; no closed-form valuation.
- Assumes GBM dynamics and discrete observation; continuous monitoring adjustments are not included.
- Quanto handling limited to correlation/vol inputs; full multi-currency simulation is not modeled.

## Pricing Methodology
- Monte Carlo GBM simulation of underlying path with discrete observations; accrual fraction is proportion of fixes within `[lower, upper]`.
- Coupon = accrual_fraction × coupon_rate × notional; discounted via instrument curve at payment date (default last observation).
- Optional quanto adjustment via correlation and FX vol when provided.

## Metrics
- PV, expected accrual fraction, and path-based Greeks (delta/vega/theta) via bump-and-revalue in MC mode.
- Observation hit ratios and distribution stats when path capture enabled.
- Scenario PVs for range shifts and vol shocks through registry bumps.

## Future Enhancements
- Add analytical approximations for narrow ranges to reduce MC runtime.
- Support stochastic volatility/jumps and correlated multi-asset ranges.
- Provide gradient/adjoint Greeks for lower-variance sensitivity estimates.
