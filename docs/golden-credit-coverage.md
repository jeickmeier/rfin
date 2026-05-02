# Credit Golden Coverage

This document tracks the credit slice of the `finstack.golden/1` rollout.

The executed framework uses instrument-specific fixture directories under
`finstack/valuations/tests/golden/data/`, matching the canonical instrument
families rather than a broad credit bucket.

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `pricing/cds` | `credit.cds` | Single-name CDS priced with hazard-rate inputs. |
| `pricing/cds_option` | `credit.cds_option` | CDS options priced with Black-76 spread-option inputs. |
| `pricing/cds_tranche` | `credit.cds_tranche` | Synthetic index tranches priced with hazard-rate inputs. |
| `pricing/structured_credit` | `fixed_income.structured_credit` | CLO/ABS style cashflow waterfall instruments. |

## Target Fixtures

- `pricing/cds/cds_5y_par_spread.json`
- `pricing/cds/cds_5y_running_upfront.json`
- `pricing/cds/cds_off_par_hazard.json`
- `pricing/cds/cds_high_yield_recovery.json`
- `pricing/cds_option/cds_option_payer_atm_3m.json`
- `pricing/cds_tranche/cdx_ig_5y_3_7_mezz.json`
- `pricing/structured_credit/clo_mezzanine_base_case.json`
- `pricing/structured_credit/abs_credit_card_senior.json`

## Metric Key Notes

- Design `dv01_spread` maps to strict metric key `risky_pv01` or
  `spread_dv01` depending on instrument support.
- Design `jtd` maps to `jump_to_default`.
- Design `recovery01` maps to `recovery_01`.
- CDS option design terms `delta_spread` and `gamma_spread` map to strict
  `delta` and `gamma` when available.
- Structured-credit waterfall details can live under `inputs.source_reference`;
  only scalar metrics present in `expected_outputs` are compared and written to
  the CSV.

## Fixture Workflow

Every new credit fixture follows the foundation workflow:

1. Use the shared pricing runner with `curves.discount`, `curves.hazard`,
   `surfaces.vol`, and optional scalar inputs.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add thin Rust and Python runner modules under `runners/`.
4. Add committed Rust fixture JSON under
   `finstack/valuations/tests/golden/data/pricing/<instrument_kind>/`.
5. Add one-line Rust tests in `finstack/valuations/tests/golden/pricing.rs`.
6. Add parametrized Python collectors.
7. Run focused Rust and Python tests, then `mise run goldens-test`.
