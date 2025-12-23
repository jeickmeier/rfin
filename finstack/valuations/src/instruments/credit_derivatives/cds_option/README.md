# CDS Option

## Features
- European options on CDS spreads with pay/receive direction, strike spread, and running coupon configuration.
- Uses underlying CDS conventions (`CDSConvention`) for accrual, schedule, and discount/hazard curve alignment.
- Supports upfront overrides and implied volatility inputs via `PricingOverrides`.

## Methodology & References
- Black-style pricing on forward CDS spread using risky annuity from the CDS pricer as the option annuity.
- Forward spread derived from hazard and discount curves; root-finding for strike/forward relationships uses Brent solver.
- Mirrors market-standard payer/receiver payoff definitions; no path-dependent features.

## Usage Example
```rust
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 5)?;
let opt = CdsOption::example();
let pv = opt.value(&market_context, as_of)?;
```

## Limitations / Known Issues
- European exercise only; Bermudan/American styles are not modeled.
- Assumes deterministic hazard curves and Black lognormal spread dynamics; no stochastic recovery or jump risk.
- Volatility smile handled through the provided surface/override only; no stochastic volatility.

## Pricing Methodology
- Computes forward CDS spread and risky annuity using underlying CDS pricer; applies Black-style option formula on spread.
- Discounting via CDS discount curve; hazard curve drives forward spread dynamics deterministically.
- Strike solving and upfront adjustments handled via Brent solver with tolerance controls.

## Metrics
- PV plus delta/vega to spread and volatility via bump-and-revalue; forward spread and risky annuity reported for attribution.
- CS01 of the underlying CDS and option PV01 through combined hazard bumps.
- Implied volatility back-solver from quoted premium/upfront.

## Future Enhancements
- Add normal/Bachelier spread model and displaced-diffusion support for deep OTM/ITM quotes.
- Incorporate stochastic recovery and smile surfaces beyond flat vol inputs.
- Provide callable/compound CDS option scaffolding and early-exercise approximations.
