# Swaption

## Features
- Options on interest rate swaps with configurable payer/receiver, strike, tenor, exercise style, and settlement (cash/physical).
- Supports Black lognormal or normal (Bachelier) volatility models plus optional SABR parameters and vol surface lookup.
- Helper methods for forward swap rate, annuity, and example builder; integrates pricing overrides (implied vol, quotes).

## Methodology & References
- Default pricing uses Black–76; if SABR params are supplied, uses SABR-implied Black vol; normal model available via `ModelKey`.
- Discounting from chosen curve; forward rate derived from swap legs (fixed vs floating) using market curves.
- Metrics and PV computed through `SimpleSwaptionBlackPricer` with deterministic curves/vols.

## Usage Example
```rust
use finstack_valuations::instruments::swaption::Swaption;

let swaption = Swaption::example();
let pv = swaption.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Bermudan exercise not modeled; structure assumes European expiry.
- No stochastic rates/vol beyond SABR-implied vol; quanto/FX effects are out of scope.
- Settlement type toggles payout only; actual underlying swap execution must be handled externally for physical settlement.

## Pricing Methodology
- Prices via Black–76 by default using forward swap rate/annuity; SABR-implied vol used when parameters provided; normal model available via pricer key.
- Discounting on chosen curve; strike rate solved from spec; exercise style assumed European.
- Vol surface lookup with clamping; optional implied vol override in pricing overrides.

## Metrics
- PV plus swaption Greeks (delta/vega/theta/rho) from Black/normal formulas; gamma via bump-and-revalue.
- DV01/CS01 inherit from underlying swap sensitivities through forward/annuity mapping.
- Implied vol solver and par/forward strike reporting for calibration.

## Future Enhancements
- Add Bermudan/cancellable swaption support with tree/LSMC methods.
- Support stochastic rate models (HW/LMM) and smile-consistent pricing beyond SABR interpolation.
- Provide callable CMS/INF structures interop and more settlement-style options.
