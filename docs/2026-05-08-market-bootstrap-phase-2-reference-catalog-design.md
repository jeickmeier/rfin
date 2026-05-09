# Market Bootstrap Phase 2 — Reference Catalog Completion

**Status:** Draft
**Date:** 2026-05-08
**Owner:** finstack/valuations + finstack-py
**Phase:** 2 of 5 (catalog completion)
**Depends on:** Phase 1 foundation
**Related specs:**
- Phase 1 — Canonical-path foundation: [2026-05-08-market-bootstrap-phase-1-foundation-design.md](2026-05-08-market-bootstrap-phase-1-foundation-design.md)
- Phase 3 — IDE autocomplete: [2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md](2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md)
- Phase 4 — Diagnostics: [2026-05-08-market-bootstrap-phase-4-diagnostics-design.md](2026-05-08-market-bootstrap-phase-4-diagnostics-design.md)

## 1. Motivation

Phase 1 ships three reference envelopes — the minimum to validate the canonical path. This phase rounds out the catalog to twelve examples covering every commonly-needed curve, surface, and snapshot type, plus a composite "full credit desk" example. It also migrates the production CDX IG 46 CDS option fixture to use the bootstrap path so it exercises calibration end-to-end rather than embedding hand-entered curve knot points.

## 2. Goals

- Twelve total reference envelope examples, each with an integration test that runs the envelope through the engine and asserts on accessor queries against the produced `MarketContext`.
- The CDX IG 46 fixture pricing golden uses `market_envelope` (introduced in Phase 1), with tolerances recomputed against the calibrated curves.
- Python notebook expanded into a full analyst-morning workflow walkthrough.

## 3. Non-Goals

- No new public APIs.
- No changes to the calibration engine or step types.
- IDE autocomplete (Phase 3) and diagnostics (Phase 4) remain deferred.

## 4. Architectural Baseline

Same as Phase 1 §4. New examples follow the two-track structure established there:

- Track A (steps): forward, single-name and CDX hazard, base correlation, swaption vol, equity vol, composite chained plan.
- Track B (initial_market): bond prices, equity spot + dividend schedule.

## 5. Scope — file-by-file

### 5.1 Remaining nine reference envelopes

Under [finstack/valuations/examples/market_bootstrap/](../finstack/valuations/examples/market_bootstrap/):

| File | Track | Step kinds | Key initial_market deps |
|---|---|---|---|
| `02_usd_3m_forward_curve.json` | A | `forward` | discount curve in initial_market |
| `04_cdx_ig_hazard.json` | A | `hazard` (index conventions: IMM dates, recovery) | discount in initial_market |
| `05_cdx_base_correlation.json` | A | `base_correlation` | discount + index hazard in initial_market |
| `06_cdx_index_vol.json` | A | `vol_surface` (SABR/credit) | discount + index hazard in initial_market |
| `07_swaption_vol_surface.json` | A | `swaption_vol` | discount + forward in initial_market |
| `08_equity_vol_surface.json` | A | `vol_surface` (SABR/equity) | discount + dividend schedule + spot in initial_market |
| `10_bond_prices.json` | B | none (empty plan) | initial_market.prices keyed by bond ID |
| `11_equity_spots_dividends.json` | B | none (empty plan) | initial_market.prices (spot) + initial_market.dividends |
| `12_full_credit_desk_market.json` | A composite | `discount` → `hazard` → `base_correlation` chained | initial_market.fx |

Each envelope file structure mirrors Phase 1: leading `description` field, full envelope JSON. Phase 3 adds `$schema` references to all envelopes including these.

[finstack/valuations/tests/calibration/reference_envelopes.rs](../finstack/valuations/tests/calibration/reference_envelopes.rs):
- Add nine integration tests, one per envelope. Each follows the Phase 1 pattern: load → execute → assert representative accessor lookups.
- Track B tests specifically demonstrate retrieval patterns:
  - `10_bond_prices.json` test: `market.get_price(&"BOND_XYZ".into())` returns the expected scalar; assert at least 2 distinct bond IDs are accessible.
  - `11_equity_spots_dividends.json` test: `market.get_price(&"AAPL".into())` returns the spot; the dividend schedule for the same name is accessible and non-empty.
  - (FX cross-rate retrieval is covered in Phase 1's `09_fx_matrix.json`.)
- Track A composite test (`12_full_credit_desk_market.json`): assert that all three calibrated curves (discount, hazard, base correlation) and the snapshot FX matrix are present and queryable in one `MarketContext`.

### 5.2 CDX IG 46 fixture migration

[finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json](../finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json):
- Replace the embedded `market.curves.discount` and `market.curves.hazard` knot snapshots with a `market_envelope` block:
  - Move S531 deposit and swap rates into a `discount` calibration step.
  - Move CDX par spreads into a `hazard` calibration step that depends on the discount curve.
  - Drop hand-entered `knot_points` for both curves.
- Keep the SABR vol surface in `initial_market.surfaces` for now (it is not currently calibrated from quotes in this fixture; could be deferred to a later sweep).
- Keep `expected_outputs` as raw Bloomberg screen values (provenance preserved).
- Recompute `actual_outputs_under_bloomberg_quadrature`, `tolerances`, and `finstack_reconciliation` notes after the bootstrap path is wired. Tolerances are remeasured from the new bootstrapped base curves, not assumed.
- Document the observed delta from the old hand-entered curves to the new bootstrapped curves in the fixture's `provenance.notes` field.

### 5.3 Notebook expansion

[finstack-py/examples/notebooks/market_bootstrap_tour.ipynb](../finstack-py/examples/notebooks/market_bootstrap_tour.ipynb) (or whatever location Phase 1 settled on):

Add sections:

- **"Build a market from raw quotes"** — uses `01_usd_discount.json`, walks through `calibrate`, residuals, `report_to_dataframe()`, accessor patterns.
- **"Compose markets"** — uses `12_full_credit_desk_market.json`, demonstrates how to chain steps with `initial_market` carrying earlier curves between steps.
- **"Snapshot-only data"** — uses `09_fx_matrix.json`, `10_bond_prices.json`, `11_equity_spots_dividends.json` to demonstrate retrieval patterns for FX cross rates, bond prices, equity spots, and dividends.
- **"Validate before solving"** — placeholder pointer to Phase 4's `dry_run` (one-line note that this section will land with diagnostics).

## 6. Acceptance Criteria

- [ ] All twelve envelope files exist under `finstack/valuations/examples/market_bootstrap/` and parse as `CalibrationEnvelope`.
- [ ] Twelve integration tests pass, each demonstrating a representative accessor query on the produced `MarketContext`.
- [ ] CDX IG 46 fixture uses `market_envelope`. Pricing golden passes with newly-calibrated base curves and remeasured tolerances. Bloomberg expected outputs unchanged; finstack reconciliation notes updated.
- [ ] Notebook has at least three additional sections covering composition, snapshot data, and accessor patterns.
- [ ] No regressions: full test suite passes.

## 7. Verification Commands

- `cargo test -p finstack-valuations --test calibration reference_envelopes`
- `cargo test -p finstack-valuations --test golden cds_option`
- `uv run pytest -v finstack-py/tests/golden/test_pricing_cds_option.py`
- `mise run all-test`

## 8. Risks

- **Equity vol envelope dependency depth.** Equity SABR fitting requires discount + dividend schedule + spot in `initial_market`. The example file gets large; consider whether to ship the dependent state inline or via a `composite` chained plan that builds dividend/spot in earlier steps. Decide during implementation; both work.
- **Base correlation pillar conventions.** CDX tranches use IMM dates and detachment points that must match the index hazard curve's pillars. Dependency between `04_cdx_ig_hazard.json` and `05_cdx_base_correlation.json` must be tested explicitly.
- **CDX fixture residual remeasurement.** A quote-bootstrapped zero-shock base curve will not reproduce the existing hand-entered knots byte-for-byte. Tolerances may need widening; document the observed delta in fixture provenance notes.
