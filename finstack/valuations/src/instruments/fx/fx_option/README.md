# FX Option

## Features
- Garman–Kohlhagen FX options with base/quote currencies, strike, settlement type, and exercise style fields.
- Uses domestic and foreign discount curves, FX vol surface, and optional implied-vol override for pricing/greeks.
- Helpers for canonical construction (`example`, `european_call`, `european_put`) plus implied-vol solver and greeks calculator.

## Methodology & References
- Garman–Kohlhagen (1983) / Black–76 style analytics with continuous foreign/domestic carry.
- Deterministic inputs from `MarketContext` (discount curves, vol surface, FX spot); no quanto or stochastic volatility.
- American/Bermudan styles are not explicitly modeled; primary path is European analytic pricing.

## Usage Example
```rust
use finstack_valuations::instruments::fx::fx_option::FxOption;

let option = FxOption::example();
let pv = option.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Assumes log-normal FX dynamics; no support for local-vol or stochastic-vol pricing.
- Quanto adjustments are not included; cross-currency risks handled via chosen curves only.
- Early-exercise behavior is not fully modeled; pricing is calibrated to European analytics.

## Pricing Methodology
- Garman–Kohlhagen (Black–76) analytic pricing with domestic/foreign carry, vol from FX surface or override.
- Time to expiry from instrument day-count; discounting via domestic curve, foreign curve used for carry.
- Exercise style primarily European; American/Bermudan not explicitly modeled in calculator.

## Metrics
- PV plus Greeks (delta/gamma/vega/theta/rho) from analytic formulas.
- Implied volatility solver and bump-and-revalue scenario metrics on spot, carry, and vol.
- DV01 on domestic curve for discounting exposure; FX delta in both base/quote terms.

## Future Enhancements
- Add American/barrier-style adjustments or link to barrier pricers for hybrids.
- Support smile-consistent local-vol/stochastic-vol models and skew-aware greeks.
- Provide quanto adjustments and proxy hedging analytics for cross-ccy exposures.
