# CDS Index

## Features

- Handles CDS indices in two modes: single-curve synthetic index pricing or constituent-level aggregation with per-name curves.
- Par-spread calculation configurable via `ParSpreadMethod` (risky annuity or full premium with accrual-on-default).
- Supports index factors, weight normalization, and standard IMM schedule conventions.

## Methodology & References

- Delegates leg PVs and risky annuity calculations to the single-name CDS pricer to maintain parity with ISDA standards.
- Constituents mode aggregates weighted single-name results; optional normalization to handle slight weight sum drift.
- Deterministic hazard/discount curves; no copula-style correlation modeled inside the pricer.

## Usage Example

```rust
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 5)?;
let index = CDSIndex::example();
let pv = index.value(&market_context, as_of)?;
let par = index.par_spread(&market_context, as_of)?;
```

## Limitations / Known Issues

- No default correlation or contagion modeling; constituents mode sums deterministic single-name valuations.
- Index roll mechanics beyond the provided schedule/series must be managed externally.
- Relies on supplied hazard and discount curves; no calibration built into the pricer.

## Pricing Methodology

- Delegates protection/premium leg PVs to single-name CDS pricer; aggregates by weights when in constituents mode.
- Par spread solved via Brent on index RPV01 with configurable denominator (risky annuity vs full premium AoD).
- Supports index factor scaling and optional weight normalization; deterministic hazard/discount curves.

## Metrics

- PV, par spread, index RPV01, upfront for given quote, and CS01 (parallel/bucketed) via constituent aggregation.
- Leg PV breakdown (premium vs protection) and accrued components.
- Weight diagnostics (sum/normalization) for data quality checks.

## Future Enhancements

- Provide roll mechanics and curve-building helpers around series rolls.
- Support stochastic spread simulation and correlation for scenario analytics.
