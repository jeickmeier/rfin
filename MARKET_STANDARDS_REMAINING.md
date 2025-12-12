# Market Standards Punch List (Remaining)

This file tracks remaining market-standards hardening items after the most recent remediations:


## Critical / High

- (none currently)

## Medium

- (none currently)

## Low (completeness / product maturity)

- Explicitly not implemented features (ensure clearly gated + documented)
  - `finstack/valuations/src/metrics/risk/var_calculator.rs:199` Taylor approximation VaR

## Completed

- XCCY is re-enabled with a market-standard design:
  - Dedicated multi-currency `XccySwap` instrument with explicit leg discounting/projection curves, FX conversion requirements, and strict calendars: `finstack/valuations/src/instruments/xccy_swap/*`.
  - XCCY discount-curve bootstrap calibrator implemented (no single-currency proxying): `finstack/valuations/src/calibration/methods/xccy.rs`.
- TRS schedule generation is now strict (no empty-schedule fallback).
- Schedule “market standard” helpers now use real calendar IDs, and FRA fixing-date logic now honors `allow_calendar_fallback`.
- Credit-adjusted PV now requires an explicit hazard curve (no implicit SP=1.0 fallback).
- Deposit cashflow generation now requires `quote_rate` (no implicit 0% quote).
- `build_dates()` is now truly non-panicking (no `expect`, safe invalid-range handling): `finstack/valuations/src/cashflow/builder/date_generation.rs`.
- Volatility conversion helpers no longer panic (fallible APIs): `finstack/valuations/src/instruments/common/models/trees/short_rate_tree.rs`.
- Base correlation calibrator uses the library-wide penalty constant: `finstack/valuations/src/calibration/methods/base_correlation.rs`.
- Swaption vol surface evaluation now has an explicit extrapolation policy (default fail-fast, opt-in clamping): `finstack/valuations/src/instruments/pricing_overrides.rs`, `finstack/valuations/src/instruments/swaption/*`.
- `finstack/valuations/src/instruments/swaption/pricer.rs:159` LSMC pricing
- `finstack/valuations/src/instruments/common/models/closed_form/heston.rs:271` full Fourier inversion