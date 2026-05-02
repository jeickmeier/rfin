# FX Option Golden Coverage

This document tracks the FX option slice of the `finstack.golden/1` rollout.

The executed framework uses the instrument-specific fixture directory
`finstack/valuations/tests/golden/data/pricing/fx_option`, matching the
canonical instrument type rather than a broad FX bucket.

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `pricing/fx_option` | `fx.fx_option` | Vanilla FX options priced with Garman-Kohlhagen inputs. |

## Target Fixtures

- `pricing/fx_option/gk_eurusd_atm_3m.json`
- `pricing/fx_option/gk_eurusd_25d_call.json`
- `pricing/fx_option/gk_usdjpy_atm_1y.json`
- `pricing/fx_option/gk_eurusd_otm_call_6m.json`

## Metric Key Notes

- Use standard scalar metric ids emitted by the pricing result: `delta`,
  `gamma`, `vega`, `theta`, `rho`, and `foreign_rho`.
- The design terms `delta_spot` and `delta_premium_adjusted` map to different
  FX market conventions. The strict metric parser currently exposes `delta`,
  not a separate premium-adjusted delta key, so premium-adjusted reference
  details belong under `inputs.source_reference`.
- Reference-only values are not compared or written to the CSV unless they also
  appear in `expected_outputs`.

## Fixture Workflow

Every new FX option fixture follows the foundation workflow:

1. Use the shared pricing runner with `curves.discount`, `fx`, and
   `surfaces.vol` market inputs.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add thin Rust and Python runner modules under `runners/`.
4. Add committed Rust fixture JSON under
   `finstack/valuations/tests/golden/data/pricing/fx_option/`.
5. Add one-line Rust tests in `finstack/valuations/tests/golden/pricing.rs`.
6. Add a parametrized Python collector.
7. Run focused Rust and Python tests, then `mise run goldens-test`.
