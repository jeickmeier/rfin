# Attribution Golden Coverage

This document tracks the attribution slice of the `finstack.golden/1` rollout.

Attribution references are deterministic textbook/formula examples. The golden
harness stores flattened scalar outputs under `inputs.actual_outputs` and
compares those values in both Rust and Python.

## Target Fixtures

- `attribution/brinson_fachler_2period.json`
- `attribution/brinson_hood_beebower.json`
- `attribution/multi_factor_ff3_attribution.json`
- `attribution/currency_local_decomposition.json`
- `attribution/contribution_to_return.json`
- `attribution/fi_carry_decomposition.json`
- `attribution/fi_curve_attribution_parallel_slope_twist.json`
- `attribution/fi_risk_based_carry_rates_credit_residual.json`

## Domain Mapping

| Fixture | Golden domain | Notes |
|---|---|---|
| Equity attribution fixtures | `attribution.equity` | Brinson, factor, currency, and contribution examples. |
| Fixed-income attribution fixtures | `attribution.fixed_income` | Carry, curve, credit, and residual decomposition examples. |

## Metric Key Notes

- Segment-level equity attribution keys use
  `<component>::<segment>::<period>`.
- Totals use `total_<component>` or `total_return`.
- Fixed-income components are expressed in return space and sum exactly to the
  fixture total within tolerance.
