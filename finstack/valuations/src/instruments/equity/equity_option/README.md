# Equity Option

## Features
- European, American, or Bermudan (fallback to American) call/put with configurable strike, expiry, contract size, and dividend yield source.
- Supports continuous-dividend Black–Scholes pricing with Leisen–Reimer tree fallback for early exercise.
- Vol surface lookup with implied-vol override, plus pricing overrides and standard discount curve inputs.

## Methodology & References
- Black–Scholes analytic pricing for European style; American priced with Leisen–Reimer binomial tree (201 steps).
- Dividend yield pulled from market scalar; deterministic rates/vols from discount and vol surfaces.
- Bermudan currently priced conservatively using the American tree due to lack of exercise schedule input.

## Usage Example
```rust
use finstack_valuations::instruments::equity::equity_option::EquityOption;

let option = EquityOption::example();
let pv = option.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Bermudan exercise schedule not modeled; American tree is used as an upper bound.
- No stochastic volatility or jumps; relies on supplied vol surface or override.
- Cash settlement vs. physical is parameterized via `settlement`; exotic payoffs (barrier/Asian) use dedicated modules.

## Pricing Methodology
- European: Black–Scholes with continuous dividend yield; American/Bermudan use Leisen–Reimer binomial tree.
- Vol from surface or override; discounting via curve; time to expiry from instrument day-count.
- Bermudan currently treated as American for conservative valuation.

## Metrics
- PV plus Greeks (delta/gamma/vega/theta/rho) from analytic (Euro) or tree (Amer/Bermudan) methods.
- Implied volatility solver; scenario PVs via bump-and-revalue on spot/vol/rates.
- Contract-size scaling and cash/physical settlement support in reporting.

## Future Enhancements
- Add explicit Bermudan exercise schedule support and early-exercise policy controls.
- Support local/stochastic volatility smile models and jump diffusion variants.
- Provide American option greeks via lattice differentiation or Barone-Adesi/Whaley approximations.
