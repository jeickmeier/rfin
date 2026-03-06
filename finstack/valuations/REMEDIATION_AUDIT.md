# Valuations Remediation Audit

## Verified remediations

- Phase 1
  - Public SIMM now requires real `Marginable` SIMM sensitivities and no longer returns an IR-only partial margin.
  - Student-t calibration fails closed in both execution and preflight instead of publishing placeholder calibrated values.
  - Expired FX touch options now require explicit observed touch state and settle from that state.

- Phase 2
  - Inflation swap CPI projection now uses the inflation curve's own date anchor rather than the discount/valuation date.
  - Zero-coupon inflation swaps retain value until adjusted payment date, not just contractual maturity.
  - CMS floating funding leg now includes the first accrual period.
  - CDS options now carry and use an explicit underlying CDS convention.
  - CDS index constituent pricing now skips defaulted names, renormalizes surviving weights, and applies upfront overrides with side-aware sign.

- Phase 3
  - Agency MBS cashflow generation now starts from the active accrual period and uses agency payment-date rules from accrual start.
  - Inflation-linked bond deflation protection now floors principal-only for `MaturityOnly` and both coupon/principal for `AllPayments`.
  - Bond-future and FI TRS approximation paths are now explicitly labeled as simplified/carry-only rather than silently reading as production-grade marks.

- Phase 4
  - Forward-curve futures seeds now subtract convexity adjustment instead of adding it.
  - Futures quotes with `vol_surface_id` but no precomputed convexity adjustment now fail closed in calibration.
  - FX digital discounting now reconstructs domestic/foreign discount factors on the option pricing clock.
  - Sobol nonzero stream splitting is explicitly rejected instead of silently using overlapping substreams.
  - Control-variate estimation now handles undersized samples and negative-rounding variance safely.

- Phase 5
  - Configured dividend-yield IDs now fail closed on missing or wrongly-typed market data in barrier, Asian, lookback, and quanto pricers.
  - Commodity Bermudan options are explicitly rejected instead of being overvalued via an American approximation.

## Regression coverage added

- `margin/calculators/im/simm.rs`
- `calibration/step_runtime.rs`
- `calibration/validation/preflight.rs`
- `calibration/targets/forward.rs`
- `instruments/fx/fx_touch_option/calculator.rs`
- `instruments/fx/fx_digital_option/calculator.rs`
- `instruments/rates/inflation_swap/types.rs`
- `instruments/rates/cms_swap/pricer.rs`
- `instruments/credit_derivatives/cds_option/pricer.rs`
- `instruments/credit_derivatives/cds_index/pricer.rs`
- `instruments/fixed_income/mbs_passthrough/pricer.rs`
- `instruments/fixed_income/inflation_linked_bond/types.rs`
- `instruments/common/models/monte_carlo/rng/sobol.rs`
- `instruments/common/models/monte_carlo/variance_reduction/control_variate.rs`
- `instruments/common/helpers.rs`
- `instruments/commodity/commodity_option/types.rs`
- `instruments/common/models/closed_form/barrier.rs`
- `instruments/exotics/barrier_option/pricer.rs`
- `instruments/rates/cap_floor/types.rs`
- `tests/instruments/cap_floor/validation/numerical.rs`
- `tests/instruments/cds/test_cds_metrics.rs`
- `covenants/forward.rs`
- `calibration/targets/inflation.rs`

## Verification status

- Passed targeted remediation regression runs for all implemented fixes.
- Passed focused smoke tests for bond-future and MBS follow-on changes.
- Passed full package suite: `cargo test -p finstack-valuations`
- Passed full package suite with Monte Carlo enabled: `cargo test -p finstack-valuations --features mc`
- Passed doctest suite: `cargo test -p finstack-valuations --doc`

## Approximation-only areas still explicit by design

- `instruments/fixed_income/bond_future/pricer.rs`
  - `calculate_model_price()` remains a simplified clean-price proxy and emits a runtime warning.
- `instruments/fixed_income/fi_trs/pricer.rs`
  - Total-return leg remains carry-only analytics and emits a runtime warning.
- `instruments/fixed_income/fi_trs/metrics/par_spread.rs`
  - Par spread remains derived from the carry-only analytics model and emits a runtime warning.
