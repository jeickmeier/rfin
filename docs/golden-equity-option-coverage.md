# Equity Option Golden Coverage

This document tracks the equity option slice of the `finstack.golden/1` rollout.

The executed framework uses the instrument-specific fixture directory
`finstack/valuations/tests/golden/data/pricing/equity_option`, matching the
canonical instrument type rather than a broad equity bucket.

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `pricing/equity_option` | `equity.equity_option` | Vanilla equity options priced with Black-Scholes inputs. |

## Target Fixtures

- `pricing/equity_option/bs_atm_call_1y.json`
- `pricing/equity_option/bs_otm_call_25d.json`
- `pricing/equity_option/bs_itm_put.json`
- `pricing/equity_option/bs_short_dated_1m.json`
- `pricing/equity_option/bs_with_dividend_yield.json`

## Metric Key Notes

- Use standard scalar metric ids emitted by the pricing result: `delta`,
  `gamma`, `vega`, `theta`, and `rho`.
- The design row `greeks` maps to the supported subset present in
  `expected_outputs` for each fixture.
- Reference-only details can live under `inputs.source_reference`; they are not
  compared or written to the CSV unless they also appear in `expected_outputs`.

## Fixture Workflow

Every new equity option fixture follows the foundation workflow:

1. Use the shared pricing runner with `prices`, `curves.discount`, and
   `surfaces.vol` market inputs.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add thin Rust and Python runner modules under `runners/`.
4. Add committed Rust fixture JSON under
   `finstack/valuations/tests/golden/data/pricing/equity_option/`.
5. Add one-line Rust tests in `finstack/valuations/tests/golden/pricing.rs`.
6. Add a parametrized Python collector.
7. Run focused Rust and Python tests, then `mise run goldens-test`.
