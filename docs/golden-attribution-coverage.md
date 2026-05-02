# Attribution Golden Coverage

This document tracks the attribution slice of the `finstack.golden/1` rollout.

Attribution references are deterministic textbook/formula examples. The golden
harness now stores component inputs plus sum/reconciliation definitions, then
computes totals in both Rust and Python instead of echoing flattened
`inputs.actual_outputs` snapshots.

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
