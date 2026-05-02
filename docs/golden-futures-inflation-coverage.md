# Futures And Inflation Golden Coverage

This document tracks the remaining futures and inflation slice of the
`finstack.golden/1` rollout.

Already-landed rows from the design table are not duplicated here:
`pricing/ir_future`, `pricing/cap_floor`, and `pricing/swaption` are covered by
their existing instrument-family directories.

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `pricing/bond_future` | `fixed_income.bond_future` | Bond futures with embedded CTD bond inputs. |
| `pricing/equity_index_future` | `equity.equity_index_future` | Equity index futures with spot, rates, and dividend-yield inputs. |
| `pricing/inflation_linked_bond` | `fixed_income.inflation_linked_bond` | Inflation-linked bonds using real discount and CPI inputs. |
| `pricing/inflation_swap` | `rates.inflation_swap` | Zero-coupon inflation swaps using CPI curve/index inputs. |

## Target Fixtures

- `pricing/bond_future/ust_ty_10y_front_month.json`
- `pricing/equity_index_future/spx_es_3m.json`
- `pricing/inflation_linked_bond/inflation_linked_bond_5y.json`
- `pricing/inflation_swap/inflation_zc_swap_5y.json`

## Metric Key Notes

- Bond future `futures_price` and `conversion_factor` are strict compared
  metrics. Gross/net basis, implied repo, CTD labels, and non-zero contract
  DV01 remain planned until exposed as strict metric ids.
- Equity index future `futures_price` and `basis` are strict compared metrics.
  The design alias `dv01_rate` maps to strict `dv01`.
- Inflation-linked bond `mod_duration` maps to strict `real_duration` if real
  duration is the available live metric.
- Inflation swap `par_breakeven_rate` maps to strict `par_rate`; `inflation_dv01`
  maps to `inflation01` when supported.

## Fixture Workflow

Every new fixture follows the foundation workflow:

1. Use the shared pricing runner with only market inputs both Rust and Python can
   build.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add thin Rust and Python runner modules under `runners/`.
4. Add committed Rust fixture JSON under
   `finstack/valuations/tests/golden/data/pricing/<instrument_kind>/`.
5. Add one-line Rust tests in `finstack/valuations/tests/golden/pricing.rs`.
6. Add parametrized Python collectors.
7. Run focused Rust and Python tests, then `mise run goldens-test`.
