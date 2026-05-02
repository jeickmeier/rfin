# Fixed Income Golden Coverage

This document tracks the fixed-income slice of the `finstack.golden/1` rollout.

The executed framework uses instrument-specific fixture directories under
`finstack/valuations/tests/golden/data/`, such as `pricing/bond`, rather than a
broad `pricing/fixed_income` bucket. This keeps Rust one-line tests and Python
parametrized collectors aligned with one instrument kind per directory.

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `pricing/bond` | `fixed_income.bond` | Government, corporate, callable, and amortizing bonds. |
| `pricing/convertible` | `fixed_income.convertible` | Convertible bonds with spot, volatility, dividend, rate, and credit inputs. |
| `pricing/term_loan` | `fixed_income.term_loan` | Institutional term loans and delayed-draw variants. |

## Metric Key Notes

| Design wording | Runner key | Notes |
|---|---|---|
| `modified_duration` | `duration_mod` | Standard metric id used by the pricing result. |
| `mod_dur` | `duration_mod` | Same as above. |
| `weighted_avg_life` | `wal` | Standard weighted-average-life metric id. |
| `effective_duration` | `duration_mod` | Assert for callable bonds only if the callable path routes modified duration to effective duration. |
| `effective_convexity` | `convexity` | Assert for callable bonds only if the callable path routes convexity to effective convexity. |
| `key_rate_dv01` | `bucketed_dv01::*` | Compare only concrete scalar keys that both Rust and Python emit. |
| `cs01_zspread` | `cs01` or `cs01::<instrument_id>` | Compare only the exact emitted metric key. |

Reference-only values can live under `inputs.source_reference`; they are not
included in the CSV unless they also appear in `expected_outputs`.

## Target Fixtures

### Bonds

- `pricing/bond/ust_2y_bullet.json`
- `pricing/bond/ust_10y_bullet.json`
- `pricing/bond/ust_30y_long_duration.json`
- `pricing/bond/corp_ig_5y_zspread.json`
- `pricing/bond/corp_hy_5y_ytm_recovery.json`
- `pricing/bond/bond_with_accrued_midperiod.json`
- `pricing/bond/corp_callable_7nc3.json`
- `pricing/bond/amortizing_bond_known_schedule.json`

### Convertible Bonds

- `pricing/convertible/conv_bond_atm_3y.json`
- `pricing/convertible/conv_bond_distressed.json`

### Term Loans

- `pricing/term_loan/term_loan_b_5y_floating.json`

## Fixture Workflow

Every new fixed-income fixture follows the foundation workflow:

1. Add or reuse shared pricing runner support for required market inputs.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add thin Rust and Python runner modules under `runners/`.
4. Add committed Rust fixture JSON under `finstack/valuations/tests/golden/data/`.
5. Add a one-line Rust test in `finstack/valuations/tests/golden/pricing.rs`.
6. Add a parametrized Python collector if the fixture directory is new.
7. Run focused Rust and Python tests, then `mise run goldens-test`.
