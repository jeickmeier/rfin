# Market Bootstrap Phase 1 — Canonical-Path Foundation

> **Superseded** in v3 envelope shape: see [2026-05-10-calibration-envelope-cleanup-design.md](2026-05-10-calibration-envelope-cleanup-design.md). References to `initial_market` in this document predate the v3 cleanup.

**Status:** Draft
**Date:** 2026-05-08
**Owner:** finstack/valuations + finstack-py + finstack-wasm
**Phase:** 1 of 5 (foundation)
**Related specs:**
- Phase 2 — Reference catalog completion: [2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md)
- Phase 3 — IDE autocomplete: [2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md](2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md)
- Phase 4 — Diagnostics: [2026-05-08-market-bootstrap-phase-4-diagnostics-design.md](2026-05-08-market-bootstrap-phase-4-diagnostics-design.md)
- Phase 5 — Fast-follow Python TypedDict: [2026-05-08-market-bootstrap-phase-5-fast-follow-design.md](2026-05-08-market-bootstrap-phase-5-fast-follow-design.md)

## 1. Motivation

Building a `MarketContext` from raw quotes is the entry point for valuations, attribution, scenarios, and portfolio analysis. The capability already exists end-to-end:

- `engine::execute(envelope)?.result.final_market` produces a fully materialized `MarketContextState`.
- `MarketContext::try_from(MarketContextState)` rehydrates the state into a live context.
- Python `calibrate(envelope_json) -> CalibrationResult` exposes `.market` as a live `MarketContext`.
- WASM has the same Rust function compiled but **does not re-export it from the JS facade**, so JavaScript users currently cannot call it.

Two more problems make this canonical-path story incomplete today:

1. Documentation does not lead with this path. Crate docs and `__init__.pyi` do not present "build a `MarketContext` from quotes" as the canonical workflow. Users find `MarketContext::from_json` first and reach for materialized snapshots when they should be calibrating from quotes.
2. Pricing golden fixtures embed materialized `MarketContext` JSON only. They do not exercise the calibration path end-to-end. A fixture that calibrates from quotes and then prices catches calibration regressions; a fixture with a hand-entered curve does not.

This phase establishes `calibrate(envelope).market` as the documented, tested, and usable canonical path across Rust, Python, and JavaScript — without adding a new function alias. (A `bootstrap_market` alias was considered and rejected; see §3.)

## 2. Goals

- `calibrate(envelope_json).market` is documented as the single canonical path to build a `MarketContext` from quotes, in Rust crate docs, Python `__init__.pyi`, and WASM TypeScript declarations.
- `calibrate` and `validateCalibrationJson` are reachable from JavaScript through the public WASM facade.
- Three reference envelopes (covering both bootstrapped and snapshot-data tracks) ship with integration tests that demonstrate full usage including accessor lookups on the resulting `MarketContext`.
- Pricing golden fixtures accept a `market_envelope` JSON block as an alternative to `market`. Both forms are mutually exclusive.
- A Python notebook scaffold exists showing the end-to-end flow.

## 3. Non-Goals and Explicit Rejections

- **No new function alias.** A `bootstrap_market(envelope_json) -> MarketContext` was considered and rejected. Rationale:
  - Functionally it is just `calibrate(envelope_json).market`. The savings are one attribute access.
  - It hides the calibration report. `CalibrationResult.market` keeps `.report_json`, residuals, RMSE adjacent to the context — that is how a user knows their curves actually fit. An alias that returns just the context encourages users to discard diagnostics they should be looking at. That is a real anti-feature for production analyst use.
  - Two ways to do the same thing splits docs and examples without value.
- Conversion of the production CDX IG 46 fixture is deferred to Phase 2.
- Reference envelopes for forward curves, base correlation, swaption vol, equity vol, bond prices, equity spots, and the full credit-desk composite are deferred to Phase 2.
- IDE autocomplete (JSON Schema, TypeScript types, Python TypedDicts) is deferred to Phases 3 and 5.
- Structured error types and rich diagnostics are deferred to Phase 4.

## 4. Architectural Baseline

The following primitives exist and are not changed by this phase:

- `CalibrationEnvelope` — JSON contract at [finstack/valuations/src/calibration/api/schema.rs](../finstack/valuations/src/calibration/api/schema.rs). Top-level fields: `schema` (must be `"finstack.calibration"`), `plan` (with `id`, `description`, `quote_sets`, `steps`, `settings`), and optional `initial_market: MarketContextState`.
- `engine::execute` at [finstack/valuations/src/calibration/api/engine.rs](../finstack/valuations/src/calibration/api/engine.rs). Returns `CalibrationResultEnvelope` whose `result.final_market: MarketContextState` is the materialized output.
- Python `calibrate(envelope_json) -> CalibrationResult` at [finstack-py/src/bindings/valuations/calibration.rs](../finstack-py/src/bindings/valuations/calibration.rs). `CalibrationResult.market -> MarketContext` is the live context, with `report_json`, `step_report_json`, `market_json`, `report_to_dataframe()` available.
- WASM `calibrate(envelopeJson)` and `validateCalibrationJson(json)` at [finstack-wasm/src/api/valuations/calibration.rs](../finstack-wasm/src/api/valuations/calibration.rs). Compiled to WASM but **not** re-exported from the JS facade.
- `MarketContext::try_from(MarketContextState)` at [finstack/core/src/market_data/context/state_serde.rs](../finstack/core/src/market_data/context/state_serde.rs).

Two-track structure of reference envelopes (carried through Phases 1 and 2):

- **Track A — bootstrapping:** quotes go in `plan.quote_sets`, are processed by `plan.steps`. Step kinds today: `discount`, `forward`, `hazard`, `inflation`, `vol_surface`, `swaption_vol`, `base_correlation`, `student_t`, `hull_white`, `cap_floor_hull_white`, `svi_surface`, `xccy_basis`, `parametric`.
- **Track B — snapshot data:** FX matrices, bond prices, equity spot prices, dividend schedules carry as `initial_market` blocks. The `MarketQuote` enum has `Fx` and `Bond` variants but no calibration step consumes them today; these are documented as snapshot-only. There is no `EquityQuote` type — equity spot prices live as scalars in `initial_market.prices`, and dividend schedules in `initial_market.dividends`.

## 5. Scope — file-by-file

### 5.1 Documentation positioning

[finstack/valuations/src/calibration/mod.rs](../finstack/valuations/src/calibration/mod.rs):
- Rewrite the crate-level doc comment to open with "Building a `MarketContext` from raw quotes," pointing at `engine::execute` and `CalibrationEnvelope`.
- Document the two-track structure (steps for bootstrapped data, `initial_market` for snapshot data).
- Cross-reference the materialized-snapshot deserialization path (`serde_json::from_str::<MarketContextState>(...)` followed by `MarketContext::try_from(state)`) as a different use case: rehydrating a previously-saved context, not building one from quotes.

[finstack-py/finstack/valuations/**init**.pyi](../finstack-py/finstack/valuations/__init__.pyi):
- Module docstring leads with "Build a `MarketContext` from quotes via `calibrate(envelope_json).market`."
- `calibrate` docstring expanded with a minimal envelope skeleton and the step-vs-snapshot distinction.

[finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts) (after 5.2 lands):
- Module-level comment block leading with the canonical path.

### 5.2 WASM facade exposure

[finstack-wasm/exports/valuations.js](../finstack-wasm/exports/valuations.js):
- Re-export `calibrate` and `validateCalibrationJson` alongside existing exports.

[finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts):
- Add: `export function calibrate(envelopeJson: string): string;` and `export function validateCalibrationJson(json: string): string;`.
- Phase 3 will replace the `string` return with typed objects via `ts_rs`.

### 5.3 Reference envelopes

Three minimum examples, under a new directory [finstack/valuations/examples/market_bootstrap/](../finstack/valuations/examples/market_bootstrap/):

| File | Track | Purpose |
|---|---|---|
| `01_usd_discount.json` | A | Single discount-step USD-OIS curve from a small set of deposit + IRS quotes. No `initial_market`. |
| `03_single_name_hazard.json` | A | Single hazard-step single-name corporate CDS curve. `initial_market` carries a USD-OIS discount curve. Demonstrates step-on-snapshot composition. |
| `09_fx_matrix.json` | B | Empty plan (`steps: []`); `initial_market.fx` populated from a small set of cross rates (e.g., EUR/USD, USD/JPY). Demonstrates `market.fx_rate("EUR", "JPY")` triangulation. |

(Numbering leaves gaps for Phase 2 examples to fit between.)

Each example includes a `plan.description` field explaining what the envelope builds and what dependencies it expects. (`CalibrationEnvelope` uses `#[serde(deny_unknown_fields)]`, so the description must live inside `plan`, not at the top level.) Phase 3 adds `$schema` to all examples.

[finstack/valuations/tests/calibration/reference_envelopes.rs](../finstack/valuations/tests/calibration/reference_envelopes.rs) — new file with three integration tests, one per example. Each test:
1. Reads the JSON file from disk via `include_str!` or runtime `std::fs::read_to_string`.
2. Deserializes as `CalibrationEnvelope`.
3. Calls `engine::execute(&envelope)?`.
4. Converts `result.final_market` to a live `MarketContext`.
5. Asserts on a representative accessor: e.g., for `01_usd_discount.json`, `market.get_discount(&"USD-OIS".into())?.discount_factor(some_date) > 0.0`. For `09_fx_matrix.json`, `market.fx_rate("EUR", "JPY")` returns a sane value triangulated through USD.

### 5.4 Golden fixture support

[finstack/valuations/tests/golden/pricing_common.rs](../finstack/valuations/tests/golden/pricing_common.rs):
- Extend the deserialization to accept either `market` (existing — materialized JSON) or `market_envelope` (new — `CalibrationEnvelope` JSON). Mutually exclusive; reject fixtures that supply both with a clear error: `pricing fixture supplied both 'market' and 'market_envelope'; specify exactly one`.
- When `market_envelope` is present, route through `engine::execute` and convert to `MarketContext` before pricing.

[finstack-py/tests/golden/runners/pricing_common.py](../finstack-py/tests/golden/runners/pricing_common.py) and [finstack-py/tests/golden/conftest.py](../finstack-py/tests/golden/conftest.py):
- Mirror the same two-shape input handling. When `market_envelope` is present, call `finstack.valuations.calibrate(json).market`.

A tiny synthetic pricing fixture under `finstack/valuations/tests/golden/data/pricing/` using `market_envelope` so this code path is covered before any real fixture is migrated. Suggested fixture: a flat-curve discount swap pricing test where the curve is bootstrapped from two quotes — minimal but exercises the full path.

### 5.5 Python notebook scaffold

Notebook location: `finstack-py/examples/notebooks/market_bootstrap_tour.ipynb`. (If the project has an established notebook location, match it; verify during implementation.)

Phase 1 ships a single-cell flow:
1. Read `01_usd_discount.json` from disk.
2. Call `result = finstack.valuations.calibrate(envelope_json)`.
3. Print `result.success`, `result.rmse`, `result.report_to_dataframe()`.
4. Access `ctx = result.market`; query a discount factor at a future date.
5. Dump materialized state via `result.market_json`.

Phase 2 expands this notebook into a fuller walkthrough.

## 6. Test Approach

- Unit-style integration tests for the three reference envelopes (5.3) — these are the primary acceptance check.
- Synthetic golden fixture using `market_envelope` exercises the runner change end-to-end (5.4).
- Existing pricing golden fixtures continue to pass under the original `market` form (no regression).

## 7. Acceptance Criteria

- [ ] `calibrate` and `validateCalibrationJson` callable from JavaScript via the public WASM exports.
- [ ] [finstack/valuations/src/calibration/mod.rs](../finstack/valuations/src/calibration/mod.rs) crate doc opens with the canonical-path narrative.
- [ ] [finstack-py/finstack/valuations/**init**.pyi](../finstack-py/finstack/valuations/__init__.pyi) module docstring leads with `calibrate(envelope).market`.
- [ ] Three example envelope JSON files exist and parse as `CalibrationEnvelope`.
- [ ] Three Rust integration tests pass: each demonstrates the produced `MarketContext` answers a typical accessor query.
- [ ] Pricing golden runners (Rust + Python) accept `market_envelope`. Both-keys-present is rejected with the documented error message.
- [ ] At least one synthetic pricing golden fixture uses `market_envelope` and passes.
- [ ] One Python notebook cell shows: read JSON file → `calibrate` → check residuals → query the resulting `MarketContext`.

## 8. Verification Commands

- `cargo test -p finstack-valuations --test calibration reference_envelopes`
- `cargo test -p finstack-valuations --test golden golden::pricing::golden_pricing_fixtures_from_existing_json_files`
- `uv run pytest -v finstack-py/tests/golden/`
- `npm --prefix finstack-wasm run test`
- `mise run all-fmt && mise run all-lint && mise run python-build && mise run all-test`

## 9. Risks

- **Notebook location convention.** [finstack-py/examples/notebooks/](../finstack-py/examples/notebooks/) already exists; new notebook lands there, matching the established pattern.
- **Synthetic golden fixture residuals.** The new `market_envelope` test fixture will produce residuals that depend on the bootstrap solver settings. Tolerances must be set after the first successful run, not assumed.
- **Mid-phase scope creep.** The temptation will be to start migrating real fixtures or polishing diagnostics in this phase. Resist; those are Phases 2 and 4.
