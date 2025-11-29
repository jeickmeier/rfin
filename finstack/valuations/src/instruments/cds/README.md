# Credit Default Swap

## Features
- Single-name CDS with configurable pay/receive leg, coupon, schedule, and accrual-on-default policy.
- Multiple protection-leg integration methods (midpoint, Gaussian quadrature, adaptive Simpson, ISDA standard) via `CDSPricerConfig`.
- Computes par spread, risky annuity (RPV01), upfront, PV01/CS01, and protection/premium leg PVs.

## Methodology & References
- Deterministic hazard-curve valuation following ISDA CDS Standard Model conventions (survival × discount integration).
- Par-spread denominator can include or exclude accrual-on-default per configuration, matching CDSW/ISDA styles.
- Root-finding for par spread and upfront uses Brent solver with tolerances controlled in `CDSPricerConfig`.

## Usage Example
```rust
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 5)?;
let cds = CreditDefaultSwap::example();
let pv = cds.value(&market_context, as_of)?;
let par_spread = cds.par_spread(&market_context, as_of)?;
```

## Limitations / Known Issues
- Assumes deterministic recovery and hazard curves; no stochastic credit or default correlation modeling.
- No quanto/currency basis handling beyond chosen discount curve.
- Does not include front-end protection toggles beyond accumulated loss inputs.

## Pricing Methodology
- Premium/protection legs projected using hazard and discount curves with accrual-on-default handled per config.
- Protection leg integrated via selectable method (midpoint, Gaussian quadrature, adaptive Simpson, ISDA standard); survival × discount integration.
- Par spread solved with Brent root-finder against risky annuity; upfront priced off clean/dirty relationship.

## Metrics
- PV (buyer/seller), par spread, risky annuity (RPV01), PV01/CS01 (parallel and bucketed).
- Accrual-on-default impact, protection/premium leg PV decomposition, expected loss.
- Upfront-to-spread conversions and clean/dirty accrual reporting.

## Future Enhancements
- Add stochastic recovery and correlation hooks; richer accrual-on-default conventions (market fallbacks).
- Extend bucketed CS01 to tenor-specific hazard bumps and credit-curve smoothing diagnostics.
