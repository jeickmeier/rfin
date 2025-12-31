# Convertible Bond

## Features

- Hybrid fixed-income/equity instrument with conversion terms (`ConversionSpec`), call/put schedules, and optional fixed or floating coupons.
- Separate discount and credit curves for Tsiveriotis–Zhang style split between debt and equity components.
- Supports voluntary/mandatory/windowed conversion, anti-dilution policies, dividend adjustments, and underlying equity linkage.

## Methodology & References

- Tree-based Tsiveriotis–Zhang (1995) convertible framework with binomial or trinomial lattices (`ConvertibleTreeType`).
- Cashflow generation reused from the bond cashflow builder; conversion, call, and put events mapped onto tree steps.
- Deterministic equity process (single-factor equity tree); no stochastic credit/equity correlation beyond curve inputs.

## Usage Example

```rust
use finstack_valuations::instruments::fixed_income::convertible::ConvertibleBond;
use finstack_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 2)?;
let cb = ConvertibleBond::example();
let pv = cb.value(&market_context, as_of)?;
```

## Limitations / Known Issues

- Tree model only; no Monte Carlo or PDE implementation.
- Assumes single equity underlying with deterministic volatility; no stochastic credit/equity correlation or jump processes.
- Complex features like contingent conversion triggers beyond the provided `ConversionPolicy` variants are out of scope.

## Pricing Methodology

- Cashflow schedule generated via bond builder (fixed/floating coupons, calls/puts) then priced on Tsiveriotis–Zhang binomial/trinomial tree.
- Splits value into cash (credit-discounted) and equity (risk-free) components; conversion, call, and put events mapped to tree nodes.
- Deterministic equity process; discount/credit curves drive debt leg; conversion ratio and policies drive optionality.

## Metrics

- PV plus tree-based Greeks (delta/gamma/vega/theta) from binomial/trinomial lattice; parity and conversion premium analytics.
- Credit DV01/CS01 via credit-curve bumps; callable/puttable option values for attribution.
- Sensitivity to conversion terms (ratio, triggers) via scenario bump hooks.

## Future Enhancements

- Add finite-difference/PDE and Monte Carlo hybrid methods for complex conversion triggers.
- Support stochastic credit/equity correlation and jump processes.
- Improve calibration helpers for implied volatility/credit from market CB quotes.
