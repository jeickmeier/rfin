# Market Bootstrap Phase 1 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish `calibrate(envelope_json).market` as the documented, tested, reachable canonical path for building a `MarketContext` from raw quotes, across Rust / Python / WASM, without adding a new function alias.

**Architecture:** Reuse the existing `engine::execute(CalibrationEnvelope) → CalibrationResultEnvelope` pipeline. Three reference envelopes (one bootstrapping, one composing on `initial_market`, one snapshot-only via `initial_market.fx`) ship with integration tests that demonstrate accessor patterns on the resulting `MarketContext`. Pricing golden runners gain a `market_envelope` input as an alternative to the existing materialized `market` block. Documentation in Rust crate docs / Python pyi / WASM declarations leads with the canonical-path narrative.

**Tech Stack:** Rust (`finstack-valuations`, `finstack-core`), PyO3 (`finstack-py`), wasm-bindgen (`finstack-wasm`), serde JSON envelopes, Jupyter for the notebook scaffold.

**Spec reference:** [`docs/2026-05-08-market-bootstrap-phase-1-foundation-design.md`](2026-05-08-market-bootstrap-phase-1-foundation-design.md)

**Commit policy:** This project's user-policy is *no commits without explicit approval*. Each task ends with a `git commit` step shown for completeness. The implementer **must confirm with the user** before running each commit, or run all commits in a single batch at end if the user prefers. Do not skip-hook (`--no-verify`) under any circumstance.

---

## File Structure

### Files to create

| Path | Responsibility |
|---|---|
| `finstack/valuations/examples/market_bootstrap/01_usd_discount.json` | Reference envelope: USD-OIS discount curve from deposit + IRS quotes (Track A bootstrapping). |
| `finstack/valuations/examples/market_bootstrap/03_single_name_hazard.json` | Reference envelope: single-name hazard curve calibrated on top of an `initial_market` discount (Track A composition). |
| `finstack/valuations/examples/market_bootstrap/09_fx_matrix.json` | Reference envelope: empty plan, FX matrix supplied via `initial_market.fx` (Track B snapshot data). |
| `finstack/valuations/tests/calibration/reference_envelopes.rs` | Three integration tests, one per reference envelope; load JSON, run engine, assert accessor lookups on the resulting `MarketContext`. |
| `finstack/valuations/tests/golden/data/pricing/market_envelope_smoke/usd_deposit_3m_envelope.json` | Tiny synthetic golden pricing fixture using `market_envelope` (clone of the existing `usd_deposit_3m.json` with materialized `market` replaced by a quote-driven `market_envelope`). |
| `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb` | Jupyter notebook scaffold demonstrating end-to-end: load envelope JSON → `calibrate` → check residuals → query `MarketContext`. |

### Files to modify

| Path | Change |
|---|---|
| `finstack-wasm/exports/valuations.js` | Add `validateCalibrationJson` and `calibrate` to the exported `valuations` namespace. |
| `finstack-wasm/index.d.ts` | Add `calibrate(envelopeJson: string): string` and `validateCalibrationJson(json: string): string` to `ValuationsNamespace`; add module-level lead comment. |
| `finstack/valuations/src/calibration/mod.rs` | Rewrite the crate-level doc opening to lead with "Building a `MarketContext` from raw quotes," documenting the two-track structure and contrasting with the materialized-snapshot deserialization path. |
| `finstack-py/finstack/valuations/__init__.pyi` | Add module-level docstring leading with the canonical path; expand `calibrate` docstring with envelope skeleton + step-vs-snapshot distinction. |
| `finstack/valuations/tests/calibration/mod.rs` | Add `mod reference_envelopes;` to register the new test file. |
| `finstack/valuations/tests/golden/pricing_common.rs` | Accept either `market` (existing) or `market_envelope` (new) in `PricingInputs`; reject if both are present; route `market_envelope` through `engine::execute`. |
| `finstack-py/tests/golden/runners/pricing_common.py` | Mirror Rust: accept either `market` or `market_envelope`; reject if both are present; route `market_envelope` through `finstack.valuations.calibrate`. |

---

## Task 1: WASM facade exposure

**Goal:** Make `calibrate` and `validateCalibrationJson` reachable from JavaScript through the public WASM exports.

**Files:**
- Modify: `finstack-wasm/exports/valuations.js:5-76`
- Modify: `finstack-wasm/index.d.ts:1377-1665` (add inside `ValuationsNamespace`)

The Rust wasm-bindgen exports already exist at `finstack-wasm/src/api/valuations/calibration.rs:13-28`; this task just re-exports them through the JS facade.

- [ ] **Step 1: Add the JS facade exports**

Open `finstack-wasm/exports/valuations.js`. Inside the `valuations` object literal, add `validateCalibrationJson` and `calibrate` near the other top-level entries (e.g., after `validateValuationResultJson`). The result of editing should look like the relevant slice below:

```js
  // Credit factor hierarchy
  CreditFactorModel: wasm.CreditFactorModel,
  CreditCalibrator: wasm.CreditCalibrator,
  LevelsAtDate: wasm.LevelsAtDate,
  PeriodDecomposition: wasm.PeriodDecomposition,
  FactorCovarianceForecast: wasm.FactorCovarianceForecast,
  decomposeLevels: wasm.decomposeLevels,
  decomposePeriod: wasm.decomposePeriod,
  validateValuationResultJson: wasm.validateValuationResultJson,
  // Calibration: build a MarketContext from raw quotes
  validateCalibrationJson: wasm.validateCalibrationJson,
  calibrate: wasm.calibrate,
  validateInstrumentJson: wasm.validateInstrumentJson,
```

- [ ] **Step 2: Add typed declarations for `index.d.ts`**

Open `finstack-wasm/index.d.ts`. In the `ValuationsNamespace` interface (currently around line 1377), add immediately after `validateValuationResultJson(json: string): string;`:

```ts
  /**
   * Validate a `CalibrationEnvelope` JSON string and return the canonical pretty-printed form.
   * Use as a pre-flight check before passing an envelope to `calibrate`.
   */
  validateCalibrationJson(json: string): string;
  /**
   * Execute a `CalibrationEnvelope` and return the full `CalibrationResultEnvelope` JSON.
   * The canonical path for building a `MarketContext` from quotes — the resulting
   * `result.final_market` is a materialized state ready for `MarketContext::try_from`
   * (Rust) or `result.market` (Python).
   */
  calibrate(envelopeJson: string): string;
```

- [ ] **Step 3: Run existing wasm tests**

The Rust wasm-bindgen module already includes integration tests at `finstack-wasm/src/api/valuations/calibration.rs:30-66` covering `validate_calibration_json_accepts_empty_plan` and `calibrate_empty_plan_succeeds`. Run them to confirm the bindings still compile and pass:

Run: `cargo test -p finstack-wasm api::valuations::calibration`
Expected: 2 passed.

- [ ] **Step 4: Run wasm-pack / npm-side typecheck (if configured)**

Run: `npm --prefix finstack-wasm run typecheck` (or whichever `tsc`-flavor command the project provides).
Expected: PASS with no `index.d.ts` errors.

If the project does not have a `typecheck` script, this step is satisfied by ensuring the file syntax is valid JavaScript / TypeScript (no syntax errors).

- [ ] **Step 5: Commit**

```bash
git add finstack-wasm/exports/valuations.js finstack-wasm/index.d.ts
git commit -m "feat(wasm): expose calibrate and validateCalibrationJson in JS facade

The Rust wasm-bindgen exports for these calibration entry points exist but
were not re-exported from finstack-wasm/exports/valuations.js, so JavaScript
users had no way to reach them. Add the exports plus typed declarations in
index.d.ts; documents calibrate as the canonical path for building a
MarketContext from quotes."
```

---

## Task 2: Reference envelope #1 — USD discount curve

**Goal:** Ship the simplest reference envelope (single discount step, no `initial_market`) and prove it round-trips through `engine::execute` to a queryable `MarketContext`.

**Files:**
- Create: `finstack/valuations/tests/calibration/reference_envelopes.rs`
- Modify: `finstack/valuations/tests/calibration/mod.rs:21-37` (add `mod reference_envelopes;`)
- Create: `finstack/valuations/examples/market_bootstrap/01_usd_discount.json`

- [ ] **Step 1: Register the new test module**

Open `finstack/valuations/tests/calibration/mod.rs`. Add `mod reference_envelopes;` to the list of module declarations. The relevant slice should read:

```rust
mod base_correlation;
mod bloomberg_accuracy;
mod bootstrap;
mod builder;
mod config;
mod explainability;
mod failure_modes;
mod finstack_config;
mod hazard_curve;
mod inflation;
mod quote_construction;
mod reference_envelopes;
mod repricing;
mod serialization;
mod swaption_vol;
mod v2_engine_smoke;
mod validation;

mod term_structures;

pub(crate) mod tolerances;
```

- [ ] **Step 2: Write the failing test for `01_usd_discount.json`**

Create `finstack/valuations/tests/calibration/reference_envelopes.rs` with:

```rust
//! Reference envelope integration tests.
//!
//! Each test loads one of the JSON examples under
//! `finstack/valuations/examples/market_bootstrap/`, runs it through the
//! calibration engine, and asserts that the resulting `MarketContext` answers
//! a typical analyst-style accessor query. The reference envelopes are the
//! canonical user-facing examples for the "build a MarketContext from quotes"
//! workflow.

use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/market_bootstrap")
}

fn load_envelope(file_name: &str) -> CalibrationEnvelope {
    let path = examples_dir().join(file_name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|err| panic!("deserialize {} as CalibrationEnvelope: {err}", path.display()))
}

fn execute(envelope: &CalibrationEnvelope) -> MarketContext {
    let result = engine::execute(envelope).expect("calibration engine succeeded");
    MarketContext::try_from(result.result.final_market.clone())
        .expect("rehydrate MarketContext from final_market state")
}

#[test]
fn example_01_usd_discount_builds_queryable_curve() {
    let envelope = load_envelope("01_usd_discount.json");
    let market = execute(&envelope);

    let curve_id = "USD-OIS".parse().expect("curve id");
    let curve = market
        .get_discount(&curve_id)
        .expect("USD-OIS discount curve present in calibrated market");

    // Discount factor at the curve base date is 1.0 by construction; a
    // forward date should produce a strictly positive DF less than 1 as a
    // sanity check that the bootstrap actually populated knot points.
    let df_today = curve.df(0.0);
    assert!(
        (df_today - 1.0).abs() < 1e-9,
        "df at t=0 should be 1.0, got {df_today}"
    );

    let df_one_year = curve.df(1.0);
    assert!(
        df_one_year > 0.0 && df_one_year < 1.0,
        "df at t=1y should be in (0, 1), got {df_one_year}"
    );
}
```

(`MarketContext::get_discount` returns the calibrated discount curve handle. If the binding name in the current code differs — e.g., `discount_curve(&id)` — the implementer should adjust to match `finstack/core/src/market_data/context/getters.rs`. The exact accessor must produce a usable `df(t)` method on the returned curve.)

- [ ] **Step 3: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_01_usd_discount_builds_queryable_curve`
Expected: FAIL with a message about the JSON file not existing (`No such file or directory`).

- [ ] **Step 4: Create the reference envelope JSON file**

Create `finstack/valuations/examples/market_bootstrap/01_usd_discount.json` with a minimal but realistic USD discount-curve envelope. Use the existing `tests/calibration/bootstrap.rs` (or `term_structures/`) source files as templates for the exact `quote_set` and `step` field shape — the implementer should base this file on a known-working in-tree calibration plan rather than guessing.

The envelope must:

1. Set `"schema": "finstack.calibration"` at the top level.
2. Define a single `quote_set` (e.g., `"usd_quotes"`) containing a small mix of deposit + IRS `RateQuote` entries. 4–6 quotes is enough to anchor short and long ends.
3. Define one `discount` step in `plan.steps` referencing the quote set, with `id` set to a curve identifier such as `"USD-OIS"` and the step's discount-curve construction params (interpolation, day_count, base date) set to project defaults.
4. Omit `initial_market` (or set it to `null`).
5. Include a `plan.description` field summarizing what the envelope builds. (`CalibrationEnvelope` uses `deny_unknown_fields`, so the description must be inside `plan`, not at the top level.)

Concrete shape to mirror — look at how an existing in-tree calibration test (e.g., `finstack/valuations/tests/calibration/bootstrap.rs`) constructs a `CalibrationEnvelope` programmatically, then serialize to JSON via `serde_json::to_string_pretty(&envelope)` and use that as the file body. This guarantees the JSON matches the exact schema.

- [ ] **Step 5: Run the test to confirm it passes**

Run: `cargo test -p finstack-valuations --test calibration example_01_usd_discount_builds_queryable_curve`
Expected: PASS.

If the engine reports the calibration failed to converge, adjust the quote rates / settings to be self-consistent (e.g., monotonically increasing rates, reasonable maturities). The test only requires successful bootstrap and a valid DF — exact rate shape is unimportant.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/tests/calibration/mod.rs \
        finstack/valuations/examples/market_bootstrap/01_usd_discount.json
git commit -m "test(valuations): add 01_usd_discount reference envelope

First of three Phase 1 reference envelopes for the canonical 'build a
MarketContext from quotes' path. Demonstrates a single discount-curve
calibration step with deposit + IRS rate quotes, exercising the full
engine::execute → MarketContext pipeline. Asserts the resulting context
answers a typical accessor query (discount factor at t=1y in (0,1))."
```

---

## Task 3: Reference envelope #2 — single-name hazard curve

**Goal:** Demonstrate composition: a hazard step running on top of a discount curve provided via `initial_market`. This is the most common analyst pattern (price single-name CDS where the rates curve already exists in the analyst's environment).

**Files:**
- Create: `finstack/valuations/examples/market_bootstrap/03_single_name_hazard.json`
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a new test)

- [ ] **Step 1: Append the failing test**

In `finstack/valuations/tests/calibration/reference_envelopes.rs`, after the first test, add:

```rust
#[test]
fn example_03_single_name_hazard_composes_on_initial_market() {
    let envelope = load_envelope("03_single_name_hazard.json");
    let market = execute(&envelope);

    // Discount curve must survive from initial_market unchanged.
    let discount_id = "USD-OIS".parse().expect("discount id");
    market
        .get_discount(&discount_id)
        .expect("discount curve carried through from initial_market");

    // Hazard curve must be produced by the calibration step.
    let hazard_id = "ISSUER-A-CDS".parse().expect("hazard id");
    let hazard = market
        .get_hazard(&hazard_id)
        .expect("hazard curve present after single-name CDS calibration");

    // Survival probability must be in (0, 1) for any positive horizon.
    let survival_one_year = hazard.survival(1.0);
    assert!(
        survival_one_year > 0.0 && survival_one_year < 1.0,
        "survival(1y) should be in (0, 1), got {survival_one_year}"
    );
}
```

(If the hazard accessor is named differently in `getters.rs` — e.g., `hazard_curve` or `get_credit_curve` — adjust to match. The test must call whichever accessor returns a curve with a `survival(t)` method.)

- [ ] **Step 2: Run the new test to confirm it fails**

Run: `cargo test -p finstack-valuations --test calibration example_03_single_name_hazard_composes_on_initial_market`
Expected: FAIL (file does not exist).

- [ ] **Step 3: Create the reference envelope JSON file**

Create `finstack/valuations/examples/market_bootstrap/03_single_name_hazard.json`. Required shape:

1. `"schema": "finstack.calibration"`.
2. `plan.description` explaining the envelope. (Top-level `"description"` is rejected by `CalibrationEnvelope`'s `deny_unknown_fields`.)
3. `plan.quote_sets` contains one set (e.g., `"issuer_a_cds_quotes"`) with 3–5 `CdsQuote` entries spanning short to long maturities (1Y, 3Y, 5Y, 7Y, 10Y is conventional).
4. `plan.steps` contains one `hazard` step referencing the CDS quote set, with `id` set to `"ISSUER-A-CDS"`, `discount_curve_id` set to `"USD-OIS"`, and `recovery_rate` set to `0.4` (or whatever the schema requires for single-name).
5. `initial_market` carries a small materialized `MarketContextState` containing exactly one curve (the `USD-OIS` discount curve, with a few knot points sufficient for hazard pricing).

To produce the JSON, the implementer should base it on existing single-name hazard calibration tests under `finstack/valuations/tests/calibration/hazard_curve.rs` — those already construct a working envelope programmatically.

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p finstack-valuations --test calibration example_03_single_name_hazard_composes_on_initial_market`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/03_single_name_hazard.json
git commit -m "test(valuations): add 03_single_name_hazard reference envelope

Demonstrates composition: a hazard calibration step running on top of an
initial_market discount curve. Exercises the most common analyst flow —
price a single-name CDS against an environment where the rates curve is
already established. Asserts both the carried-through discount and the
newly-built hazard curve are accessible via standard MarketContext getters."
```

---

## Task 4: Reference envelope #3 — FX matrix snapshot

**Goal:** Demonstrate Track B (snapshot-only data via `initial_market`). The plan is empty; the FX matrix is supplied in `initial_market.fx`. Demonstrates `market.fx_rate(...)` triangulation.

**Files:**
- Create: `finstack/valuations/examples/market_bootstrap/09_fx_matrix.json`
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a third test)

- [ ] **Step 1: Append the failing test**

In `finstack/valuations/tests/calibration/reference_envelopes.rs`, add:

```rust
#[test]
fn example_09_fx_matrix_supports_cross_rate_lookup() {
    let envelope = load_envelope("09_fx_matrix.json");
    let market = execute(&envelope);

    let fx = market.fx().expect("fx matrix populated from initial_market");

    // Direct quote: EUR/USD must equal the snapshot value (within fp slack).
    let eur_usd = fx
        .rate("EUR", "USD")
        .expect("EUR/USD direct quote");
    assert!(
        eur_usd > 0.5 && eur_usd < 2.0,
        "EUR/USD should be a sane FX rate, got {eur_usd}"
    );

    // Triangulated cross: EUR/JPY must derive from EUR/USD * USD/JPY.
    let eur_jpy = fx
        .rate("EUR", "JPY")
        .expect("EUR/JPY triangulated through pivot USD");
    assert!(
        eur_jpy > 0.0,
        "EUR/JPY triangulation should produce a positive rate, got {eur_jpy}"
    );
}
```

(The `fx()` accessor and `rate(base, quote)` method names should match the actual `MarketContext` / `FxMatrix` API. Verify against `finstack/core/src/market_data/context/getters.rs` and the FX-matrix module — adjust the test code to match the real method names. The test's intent is: assert that an `initial_market.fx` block produces a queryable `FxMatrix` that supports both direct quotes and triangulated cross rates.)

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_09_fx_matrix_supports_cross_rate_lookup`
Expected: FAIL (file does not exist).

- [ ] **Step 3: Create the reference envelope JSON file**

Create `finstack/valuations/examples/market_bootstrap/09_fx_matrix.json` with shape:

```json
{
  "schema": "finstack.calibration",
  "plan": {
    "id": "fx_snapshot",
    "description": "Snapshot-only example: FX matrix supplied via initial_market.fx, with no calibration steps. Demonstrates how to express FX cross rates today and how to triangulate via the pivot currency.",
    "quote_sets": {},
    "steps": [],
    "settings": {}
  },
  "initial_market": {
    "version": 2,
    "curves": [],
    "fx": {
      "config": {
        "pivot_currency": "USD",
        "enable_triangulation": true,
        "cache_capacity": 256
      },
      "quotes": [
        { "base": "EUR", "quote": "USD", "rate": 1.0850 },
        { "base": "USD", "quote": "JPY", "rate": 152.40 },
        { "base": "GBP", "quote": "USD", "rate": 1.2660 }
      ]
    },
    "surfaces": [],
    "prices": {},
    "series": [],
    "inflation_indices": [],
    "dividends": [],
    "credit_indices": [],
    "fx_delta_vol_surfaces": [],
    "vol_cubes": [],
    "collateral": {}
  }
}
```

The `quotes` array shape (`{base, quote, rate}`) must match the exact `FxMatrixState` serde representation. The implementer should verify against `finstack/core/src/market_data/context/state_serde.rs:200-220` (the `FxMatrixState` struct) and adjust field names if needed. The reference fixture `finstack/valuations/tests/golden/data/pricing/deposit/usd_deposit_3m.json:93-100` shows the exact serde shape with `quotes: []` — populate accordingly.

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p finstack-valuations --test calibration example_09_fx_matrix_supports_cross_rate_lookup`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/09_fx_matrix.json
git commit -m "test(valuations): add 09_fx_matrix reference envelope

Demonstrates the snapshot-only track: empty calibration plan, FX matrix
supplied via initial_market.fx with three direct quotes (EUR/USD, USD/JPY,
GBP/USD). Test asserts both direct lookup and triangulated cross-rate
retrieval (EUR/JPY via the USD pivot). Establishes the pattern for
documenting non-bootstrapped market data in the canonical envelope shape."
```

---

## Task 5: Golden fixture support — Rust runner

**Goal:** Teach the Rust pricing-fixture runner to accept either `market` (existing) or `market_envelope` (new); reject if both keys are present; route `market_envelope` through `engine::execute` and `MarketContext::try_from` before pricing. Validate with a tiny synthetic fixture.

**Files:**
- Modify: `finstack/valuations/tests/golden/pricing_common.rs:9-55`
- Create: `finstack/valuations/tests/golden/data/pricing/market_envelope_smoke/usd_deposit_3m_envelope.json`

- [ ] **Step 1: Write the failing test**

Append a new test to `finstack/valuations/tests/golden/pricing_common.rs` (or wherever the project's golden runner unit tests live — the implementer should mirror existing test placement):

```rust
#[cfg(test)]
mod market_envelope_input_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pricing_inputs_reject_when_both_market_and_market_envelope_supplied() {
        let inputs = json!({
            "valuation_date": "2026-04-30",
            "model": "discounting",
            "metrics": [],
            "instrument_json": {},
            "market": {},
            "market_envelope": {}
        });
        let parsed: Result<PricingInputs, _> = serde_json::from_value(inputs);
        let err = parsed
            .err()
            .expect("must reject fixtures that supply both 'market' and 'market_envelope'");
        let msg = err.to_string();
        assert!(
            msg.contains("market") && msg.contains("market_envelope"),
            "error message should name both fields, got: {msg}"
        );
    }
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test golden market_envelope_input_tests`
Expected: FAIL — the existing `PricingInputs` struct accepts an unknown `market_envelope` field silently or panics on `market` deserialization, depending on serde mode.

- [ ] **Step 3: Update `PricingInputs` to support both forms**

Modify `finstack/valuations/tests/golden/pricing_common.rs`. Replace the existing `PricingInputs` struct and `run_pricing_fixture` function with:

```rust
//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::schema::GoldenFixture;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_valuations::pricer::price_instrument_json_with_metrics;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PricingInputs {
    valuation_date: String,
    model: String,
    metrics: Vec<String>,
    instrument_json: serde_json::Value,
    /// Materialized MarketContext JSON (snapshot form). Mutually exclusive with `market_envelope`.
    #[serde(default)]
    market: Option<MarketContext>,
    /// CalibrationEnvelope JSON (quote-driven form). Mutually exclusive with `market`.
    #[serde(default)]
    market_envelope: Option<CalibrationEnvelope>,
    #[serde(flatten)]
    _extra: serde_json::Map<String, serde_json::Value>,
}

impl PricingInputs {
    fn resolve_market(&self) -> Result<MarketContext, String> {
        match (&self.market, &self.market_envelope) {
            (Some(_), Some(_)) => Err(
                "pricing fixture supplied both 'market' and 'market_envelope'; specify exactly one"
                    .to_string(),
            ),
            (Some(m), None) => Ok(m.clone()),
            (None, Some(env)) => {
                let result = engine::execute(env)
                    .map_err(|err| format!("calibrate market_envelope: {err}"))?;
                MarketContext::try_from(result.result.final_market.clone())
                    .map_err(|err| format!("rehydrate calibrated market: {err}"))
            }
            (None, None) => Err(
                "pricing fixture must supply either 'market' or 'market_envelope'".to_string(),
            ),
        }
    }
}

/// Price an instrument fixture that follows the common pricing input contract.
pub(crate) fn run_pricing_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    crate::golden::source_validation::validate_source_validation_fixture(
        "pricing runner",
        fixture,
    )?;

    let inputs: PricingInputs = serde_json::from_value(fixture.inputs.clone())
        .map_err(|err| format!("parse pricing inputs: {err}"))?;
    let market = inputs.resolve_market()?;
    let instrument_json = serde_json::to_string(&inputs.instrument_json)
        .map_err(|err| format!("serialize instrument_json: {err}"))?;

    let result = price_instrument_json_with_metrics(
        &instrument_json,
        &market,
        &inputs.valuation_date,
        &inputs.model,
        &inputs.metrics,
        None,
    )
    .map_err(|err| format!("price instrument JSON: {err}"))?;

    let mut actuals = BTreeMap::new();
    for metric in fixture.expected_outputs.keys() {
        let value = if metric == "npv" {
            result.value.amount()
        } else {
            *result
                .measures
                .get(metric.as_str())
                .ok_or_else(|| format!("result missing metric '{metric}'"))?
        };
        actuals.insert(metric.clone(), value);
    }
    Ok(actuals)
}
```

Notes:
- Removing `MarketContext` from the deserialize position (now inside `Option<MarketContext>`) is consistent with the new optionality.
- `#[serde(deny_unknown_fields)]` plus the explicit `_extra` field structure protects against typos. If existing fixtures use other auxiliary fields not in this struct, the implementer should remove `deny_unknown_fields` and rely on the both-present check alone — verify by running the full golden suite after this change.
- The mutual-exclusion check produces a clear human-readable error and is also raised when neither is provided.

- [ ] **Step 4: Run the unit test to confirm it passes**

Run: `cargo test -p finstack-valuations --test golden pricing_inputs_reject_when_both`
Expected: PASS.

- [ ] **Step 5: Run the full existing golden pricing suite (regression check)**

Run: `cargo test -p finstack-valuations --test golden golden::pricing`
Expected: All existing fixtures pass — they continue to use the `market` form unchanged.

If any fixtures fail because of the `deny_unknown_fields` change, remove the `#[serde(deny_unknown_fields)]` attribute from `PricingInputs` (the mutual-exclusion check still catches the both-present case). Re-run.

- [ ] **Step 6: Create the synthetic `market_envelope` fixture**

Clone `finstack/valuations/tests/golden/data/pricing/deposit/usd_deposit_3m.json` to `finstack/valuations/tests/golden/data/pricing/market_envelope_smoke/usd_deposit_3m_envelope.json`. In the copy:

1. Update `name` to `"usd_deposit_3m_envelope"` and `description` to "Phase-1 smoke fixture: same deposit + expected outputs as `deposit/usd_deposit_3m.json`, but the discount curve is built via market_envelope rather than a materialized snapshot."
2. Replace `inputs.market` with `inputs.market_envelope`. The envelope's `initial_market` should embed the same materialized USD-OIS discount curve from the original fixture (this preserves expected outputs exactly — the calibration plan can be empty, OR you can move the curve construction into a `discount` step. For Phase-1 simplicity, embed in `initial_market`):

```json
"market_envelope": {
  "schema": "finstack.calibration",
  "description": "Carry through the original USD-OIS discount curve via initial_market for byte-identical pricing.",
  "plan": {
    "id": "smoke_carry_through",
    "description": null,
    "quote_sets": {},
    "steps": [],
    "settings": {}
  },
  "initial_market": {
    "version": 2,
    "curves": [
      { "type": "discount", "id": "USD-OIS", "base": "2026-04-30", "day_count": "Act365F", "knot_points": [[0.0,1.0],[0.25,0.99],[0.5,0.9802],[1.0,0.9608],[2.0,0.9231],[5.0,0.8187]], "interp_style": "log_linear", "extrapolation": "flat_forward", "min_forward_rate": null, "allow_non_monotonic": false, "min_forward_tenor": 1e-06, "rate_calibration": null }
    ],
    "fx": { "config": { "pivot_currency": "USD", "enable_triangulation": true, "cache_capacity": 256 }, "quotes": [] },
    "surfaces": [],
    "prices": {},
    "series": [],
    "inflation_indices": [],
    "dividends": [],
    "credit_indices": [],
    "fx_delta_vol_surfaces": [],
    "vol_cubes": [],
    "collateral": {}
  }
}
```

3. Keep `expected_outputs` and `tolerances` identical to the original fixture — by carrying the same materialized curve via `initial_market`, the pricing math is unchanged and outputs match exactly.

(A pure quote-driven version of this fixture, where the discount curve is bootstrapped from a single deposit rate, is a Phase-2 follow-up. Phase 1's synthetic fixture only needs to exercise the `market_envelope` deserialization and engine-execution path, not produce a quote-bootstrapped curve.)

4. Update `provenance.description`/`source_detail` to reflect the smoke purpose. Keep `as_of`, `last_reviewed_*`, `regen_command`, `screenshots: []`.

- [ ] **Step 7: Run the new fixture through the existing golden runner**

The existing golden test discovery will pick up the new file automatically (it's under `tests/golden/data/pricing/`). Verify by running:

Run: `cargo test -p finstack-valuations --test golden golden_pricing_fixtures_from_existing_json_files -- --nocapture | grep usd_deposit_3m_envelope`
Expected: the new fixture is run and passes (same expected outputs as the original).

If the test name pattern is different in the runner, the implementer should run:
Run: `cargo test -p finstack-valuations --test golden -- --nocapture` and grep for `market_envelope_smoke` / `usd_deposit_3m_envelope`.

- [ ] **Step 8: Commit**

```bash
git add finstack/valuations/tests/golden/pricing_common.rs \
        finstack/valuations/tests/golden/data/pricing/market_envelope_smoke/
git commit -m "feat(valuations): pricing golden runners accept market_envelope input

Pricing fixtures may now supply 'market_envelope' (a CalibrationEnvelope
JSON block) as an alternative to the existing 'market' (materialized
MarketContext JSON). The runner routes 'market_envelope' through
engine::execute → MarketContext::try_from before pricing. Mutually
exclusive: providing both is rejected with a clear error. Includes a
smoke fixture that mirrors deposit/usd_deposit_3m.json via the new path."
```

---

## Task 6: Golden fixture support — Python runner

**Goal:** Mirror the Rust runner change in Python so the same fixtures work end-to-end across both binding layers.

**Files:**
- Modify: `finstack-py/tests/golden/runners/pricing_common.py:1-46`

- [ ] **Step 1: Write the failing test**

The implementer should locate the Python golden-runner unit-test file (likely `finstack-py/tests/golden/test_pricing_runners.py` or a similarly-named module). If no test for the runner exists yet, create one. Add:

```python
import json
import pytest

from tests.golden.runners.pricing_common import run_pricing_fixture
from tests.golden.schema import GoldenFixture


def _make_fixture(inputs: dict) -> GoldenFixture:
    """Construct a minimal GoldenFixture object for unit-testing the runner."""
    return GoldenFixture.model_validate({
        "schema_version": "finstack.golden/1",
        "name": "test",
        "domain": "test",
        "description": "test",
        "provenance": {
            "as_of": "2026-04-30",
            "source": "synthetic",
            "source_detail": "unit test",
            "captured_by": "pytest",
            "captured_on": "2026-04-30",
            "last_reviewed_by": "pytest",
            "last_reviewed_on": "2026-04-30",
            "review_interval_months": 6,
            "regen_command": "n/a",
            "screenshots": [],
        },
        "inputs": inputs,
        "expected_outputs": {"npv": 0.0},
        "tolerances": {"npv": {"abs": 1e-6}},
    })


def test_pricing_inputs_reject_when_both_market_and_market_envelope():
    fixture = _make_fixture({
        "valuation_date": "2026-04-30",
        "model": "discounting",
        "metrics": [],
        "instrument_json": {},
        "market": {},
        "market_envelope": {},
    })
    with pytest.raises(ValueError, match="market.*market_envelope"):
        run_pricing_fixture(fixture)
```

(If the project's `GoldenFixture` is not a Pydantic model, adjust construction to match. Use the same fixture-loading approach the existing pricing test uses.)

- [ ] **Step 2: Run the failing test**

Run: `uv run pytest -v finstack-py/tests/golden/test_pricing_runners.py::test_pricing_inputs_reject_when_both_market_and_market_envelope`
Expected: FAIL — the existing runner only handles `inputs["market"]`.

- [ ] **Step 3: Update `pricing_common.py`**

Replace the contents of `finstack-py/tests/golden/runners/pricing_common.py` with:

```python
"""Shared pricing helpers for instrument-level golden fixtures."""

from __future__ import annotations

import json

from finstack.core.market_data import MarketContext

from finstack.valuations import (
    ValuationResult,
    calibrate,
    price_instrument_with_metrics,
)
from tests.golden.pricing_validation import validated_instrument_json
from tests.golden.runners import validate_source_validation_fixture
from tests.golden.schema import GoldenFixture


def _resolve_market(inputs: dict) -> MarketContext:
    """Return a MarketContext from either the 'market' or 'market_envelope' key.

    Mutually exclusive: the fixture must provide exactly one. 'market' is the
    materialized MarketContext JSON (snapshot); 'market_envelope' is a
    CalibrationEnvelope routed through the calibration engine.
    """
    has_market = "market" in inputs
    has_envelope = "market_envelope" in inputs
    if has_market and has_envelope:
        raise ValueError(
            "pricing fixture supplied both 'market' and 'market_envelope'; "
            "specify exactly one"
        )
    if has_market:
        return MarketContext.from_json(json.dumps(inputs["market"]))
    if has_envelope:
        result = calibrate(json.dumps(inputs["market_envelope"]))
        return result.market
    raise ValueError(
        "pricing fixture must supply either 'market' or 'market_envelope'"
    )


def run_pricing_fixture(fixture: GoldenFixture) -> dict[str, float]:
    """Run one common pricing fixture through the Python bindings."""
    validate_source_validation_fixture("pricing runner", fixture)

    inputs = fixture.inputs
    market = _resolve_market(inputs)
    instrument_json = validated_instrument_json(inputs["instrument_json"])
    result_json = price_instrument_with_metrics(
        instrument_json,
        market,
        inputs["valuation_date"],
        model=inputs["model"],
        metrics=list(inputs["metrics"]),
    )
    result = ValuationResult.from_json(result_json)

    actuals: dict[str, float] = {}
    for metric in fixture.expected_outputs:
        if metric == "npv":
            actuals[metric] = float(result.price)
            continue
        value = result.get_metric(metric)
        if value is None:
            raise ValueError(f"result missing metric {metric!r}")
        actuals[metric] = float(value)
    return actuals


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run a fixture that follows the shared pricing input contract."""
    return run_pricing_fixture(fixture)
```

- [ ] **Step 4: Run the unit test to confirm it passes**

Run: `uv run pytest -v finstack-py/tests/golden/test_pricing_runners.py::test_pricing_inputs_reject_when_both_market_and_market_envelope`
Expected: PASS.

- [ ] **Step 5: Run the full existing golden pricing suite (regression check)**

Run: `uv run pytest -v finstack-py/tests/golden/`
Expected: All existing fixtures pass.

The synthetic `market_envelope` fixture from Task 5 should also be discovered and passed by the Python runner (Python golden discovery pattern matches the Rust one — same JSON files, same outputs).

- [ ] **Step 6: Commit**

```bash
git add finstack-py/tests/golden/runners/pricing_common.py \
        finstack-py/tests/golden/test_pricing_runners.py
git commit -m "feat(finstack-py): pricing golden runners accept market_envelope input

Mirrors the Rust runner change: pricing fixtures may supply
'market_envelope' (a CalibrationEnvelope JSON block) as an alternative to
the existing 'market' (materialized MarketContext). The Python runner
routes 'market_envelope' through finstack.valuations.calibrate before
pricing. Mutually exclusive; both-present is rejected with a clear error."
```

---

## Task 7: Documentation — Rust crate doc

**Goal:** Rewrite the `finstack/valuations/src/calibration/mod.rs` opening so it leads with "build a MarketContext from raw quotes" via `engine::execute` / `CalibrationEnvelope`, documents the two-track structure, and contrasts with the materialized-snapshot deserialization path.

**Files:**
- Modify: `finstack/valuations/src/calibration/mod.rs:1-73`

- [ ] **Step 1: Rewrite the opening doc comment**

Replace lines 1 through 73 (everything from the `//! Calibration framework...` opening through the existing `# References` section) with:

```rust
//! Calibration framework — the canonical path to build a `MarketContext` from
//! raw market quotes.
//!
//! # Building a MarketContext from quotes
//!
//! The supported workflow is JSON-in / `MarketContext`-out:
//!
//! ```rust
//! use finstack_valuations::calibration::api::{engine, schema::CalibrationEnvelope};
//! use finstack_core::market_data::context::MarketContext;
//!
//! # let envelope_json = r#"{"schema":"finstack.calibration","plan":{"id":"empty","description":null,"quote_sets":{},"steps":[],"settings":{}},"initial_market":null}"#;
//! let envelope: CalibrationEnvelope =
//!     serde_json::from_str(envelope_json).expect("parse envelope");
//! let result = engine::execute(&envelope).expect("calibration succeeded");
//! let market = MarketContext::try_from(result.result.final_market.clone())
//!     .expect("rehydrate market");
//! // `market` is now ready for valuations, attribution, scenarios, portfolio analysis.
//! # let _ = market;
//! ```
//!
//! Python and JavaScript users get the same surface: `finstack.valuations.calibrate(json).market`
//! returns a `MarketContext`; the `CalibrationResult` wrapper additionally exposes per-step
//! residuals and a plan-level report next to the context, so users can verify their curves
//! actually fit.
//!
//! See `finstack/valuations/examples/market_bootstrap/` for canonical envelope JSON examples
//! covering discount curves, hazard curves layered on `initial_market`, and FX matrices
//! supplied as snapshot data.
//!
//! # Two-track envelope structure
//!
//! A `CalibrationEnvelope` carries quotes in two complementary places:
//!
//! - **Track A — bootstrapping (`plan.quote_sets` + `plan.steps`).** Quotes that drive a
//!   solver — rates, CDS, swaptions, vols, tranche spreads, etc. Each `step` reads its
//!   `quote_set` and produces a curve or surface added to the in-progress context.
//!   Step kinds: `discount`, `forward`, `hazard`, `inflation`, `vol_surface`,
//!   `swaption_vol`, `base_correlation`, `student_t`, `hull_white`, `cap_floor_hull_white`,
//!   `svi_surface`, `xccy_basis`, `parametric`.
//! - **Track B — snapshot data (`initial_market`).** FX matrices, bond prices, equity
//!   spot prices, and dividend schedules are not bootstrapped today — they are supplied
//!   as materialized state. The `MarketQuote` enum has `Fx` and `Bond` variants for
//!   documentation/persistence purposes, but no calibration step consumes them; pass
//!   them via `initial_market.fx`, `initial_market.prices`, and `initial_market.dividends`.
//!
//! Both tracks are valid in the same envelope; the engine merges `initial_market`
//! into the working context before running steps.
//!
//! # When to use `MarketContext::try_from(MarketContextState)` directly
//!
//! `MarketContext::try_from(state)` (paired with `serde_json::from_str::<MarketContextState>`)
//! is the materialized-snapshot deserializer — it rehydrates a *previously-saved*
//! `MarketContext`. It does **not** build one from quotes. Use the calibration path
//! (above) for quote-driven construction; reserve direct deserialization for replaying
//! an already-calibrated context (e.g., from a saved snapshot, a downstream consumer,
//! or a regression-test fixture).
//!
//! # Documentation Rules For Calibration APIs
//!
//! Calibration docs should make three things explicit:
//!
//! - **Which quotes and conventions are assumed**: quote style, day count, curve
//!   time basis, interpolation, and market-standard construction choices should be
//!   stated near the public API that uses them.
//! - **Which tolerance is being discussed**: solver convergence tolerances and
//!   post-solve validation tolerances are distinct and should not be conflated.
//! - **Which canonical source applies**: model-heavy and convention-heavy APIs
//!   should include `# References` sections pointing to `docs/REFERENCES.md`.
//!
//! # Features
//! - **Plan-Driven API**: Uses `"finstack.calibration"` schema for structured calibration plans.
//! - **Flexible Solvers**: Supports both sequential bootstrapping and global optimization (Newton/LM).
//! - **Market Standards**: Implements post-2008 multi-curve frameworks and strict pricing conventions.
//! - **Extensible Architecture**: Easy to add new instrument types and calibration targets.
//!
//! # See Also
//! - `api` for the plan schema and engine.
//! - `solver` for the underlying numerical solvers.
//! - [`crate::market::quotes`] for market data representation.
//!
//! # References
//!
//! - Multi-curve discounting and construction: `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`
//! - Curve interpolation: `docs/REFERENCES.md#hagan-west-monotone-convex`
//! - Core rates/derivatives background: `docs/REFERENCES.md#hull-options-futures`
```

Leave the rest of the file (the `pub mod api;` declarations, re-exports, etc.) unchanged.

- [ ] **Step 2: Build the docs and verify**

Run: `cargo doc -p finstack-valuations --no-deps`
Expected: PASS (no doc-test or rustdoc errors). The example doctest in the new opening must compile under `# fn example(envelope_json: &str) -> finstack_core::Result<()> { ... # Ok(()) }`.

- [ ] **Step 3: Run the doctest explicitly**

Run: `cargo test -p finstack-valuations --doc calibration`
Expected: PASS (the new code-block compiles and runs).

If the doctest fails because `engine::execute` returns a wrapped result type that requires a different access pattern, adjust the example to match — but keep the spirit (envelope → engine → context).

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/calibration/mod.rs
git commit -m "docs(valuations): lead crate doc with canonical MarketContext path

Rewrite finstack/valuations/src/calibration/mod.rs opening to lead with
'build a MarketContext from raw quotes' via engine::execute and
CalibrationEnvelope. Document the two-track envelope structure (Track A:
bootstrapping via steps; Track B: snapshot data via initial_market) and
contrast with MarketContext::try_from(state) which is for replaying
already-calibrated contexts, not building from quotes."
```

---

## Task 8: Documentation — Python `__init__.pyi`

**Goal:** Add a module-level docstring leading with the canonical path; expand the `calibrate` docstring with an envelope skeleton + step-vs-snapshot distinction.

**Files:**
- Modify: `finstack-py/finstack/valuations/__init__.pyi:1-72` (module docstring)
- Modify: `finstack-py/finstack/valuations/__init__.pyi:1150-1173` (`calibrate` docstring)

- [ ] **Step 1: Replace the module docstring**

In `finstack-py/finstack/valuations/__init__.pyi`, replace the existing single-line opening `"""Instrument pricing, risk metrics, and P&L attribution."""` with:

```python
"""Instrument pricing, risk metrics, P&L attribution, and market-context bootstrapping.

The canonical path to build a :class:`finstack.core.market_data.MarketContext`
from raw market quotes is :func:`calibrate`:

    >>> import json
    >>> from finstack.valuations import calibrate
    >>> envelope = {
    ...     "schema": "finstack.calibration",
    ...     "plan": {
    ...         "id": "usd_curves",
    ...         "quote_sets": {"usd_quotes": [...]},  # MarketQuote entries
    ...         "steps": [{"id": "USD-OIS", "quote_set": "usd_quotes",
    ...                    "kind": "discount", ...}],
    ...         "settings": {},
    ...     },
    ...     "initial_market": None,
    ... }
    >>> result = calibrate(json.dumps(envelope))  # doctest: +SKIP
    >>> result.success         # doctest: +SKIP
    True
    >>> result.rmse            # doctest: +SKIP    # check the curves actually fit
    1.2e-9
    >>> ctx = result.market    # doctest: +SKIP    # ready for pricing/attribution

The :class:`CalibrationResult` wrapper carries the :class:`MarketContext` next
to per-step residuals (:meth:`step_report_json`, :meth:`report_to_dataframe`)
so users can verify their curves actually fit before consuming them downstream.

A `CalibrationEnvelope` carries quotes in two complementary places:

- **Bootstrapping** — ``plan.quote_sets`` + ``plan.steps``. Quotes that drive
  a solver (rates, CDS, swaptions, vols, tranches, etc.) live here. Each step
  reads its quote set and produces a curve or surface.
- **Snapshot data** — ``initial_market``. FX matrices, bond prices, equity spot
  prices, and dividend schedules are not bootstrapped today — pass them via
  ``initial_market.fx``, ``initial_market.prices``, ``initial_market.dividends``.

Reference envelope JSON examples covering both tracks live under
``finstack/valuations/examples/market_bootstrap/`` in the repository.

This module also exposes pricing (:func:`price_instrument`,
:func:`price_instrument_with_metrics`), P&L attribution
(:func:`attribute_pnl`), risk decomposition (:func:`decompose_factor_risk`),
SABR / Black-Scholes primitives, and credit-factor hierarchy tooling.
"""
```

- [ ] **Step 2: Expand the `calibrate` docstring**

Locate the `def calibrate(json: str) -> CalibrationResult:` declaration around line 1150–1173. Replace its docstring with:

```python
    """Build a :class:`MarketContext` from raw market quotes — the canonical entry point.

    Accepts a JSON-serialized ``CalibrationEnvelope``. The envelope carries
    quotes in two complementary places:

    - ``plan.quote_sets`` + ``plan.steps`` — quote-driven calibration steps
      (discount, forward, hazard, vol surface, swaption vol, base correlation,
      etc.). Each step reads its named quote set and produces a curve/surface.
    - ``initial_market`` — pre-built / snapshot data (FX matrices, bond prices,
      equity spot prices, dividend schedules). FX and Bond ``MarketQuote``
      variants exist for documentation but are not consumed by any calibration
      step today; pass them via ``initial_market.fx``, ``initial_market.prices``,
      ``initial_market.dividends``.

    Args:
        json: JSON-serialized ``CalibrationEnvelope`` (schema string is
            ``"finstack.calibration"``).

    Returns:
        :class:`CalibrationResult` with:
            - ``.market`` — the live :class:`MarketContext` (use this for
              pricing, attribution, scenarios, portfolio).
            - ``.market_json`` — same context as a JSON snapshot for
              persistence or comparison.
            - ``.report_json`` / ``.step_report_json(id)`` /
              ``.report_to_dataframe()`` — diagnostics. Always check
              ``.success`` and ``.rmse`` before relying on the produced market.
            - ``.iterations``, ``.max_residual``, ``.step_ids`` — summary stats.

    Raises:
        ValueError: If the JSON is not a valid envelope, or if calibration
            fails (e.g., missing dependency, solver non-convergence).

    Example:
        >>> import json as _json
        >>> from finstack.valuations import calibrate
        >>> result = calibrate(_json.dumps(envelope))  # doctest: +SKIP
        >>> assert result.success and result.rmse < 1e-6  # doctest: +SKIP
        >>> curve = result.market.get_discount("USD-OIS")  # doctest: +SKIP
        >>> price_json = price_instrument(inst_json, result.market_json,
        ...                                "2026-05-08")  # doctest: +SKIP

    See Also:
        - ``finstack/valuations/examples/market_bootstrap/`` — reference
          envelope JSON files (discount curve, single-name hazard, FX matrix).
        - :func:`validate_calibration_json` — pre-flight envelope check.
    """
```

- [ ] **Step 3: Run the type-stub linter / build**

Run: `mise run python-build` (or `uv run mypy finstack-py/finstack/valuations/__init__.pyi`, depending on project conventions).
Expected: PASS — no syntax or stub errors.

- [ ] **Step 4: Commit**

```bash
git add finstack-py/finstack/valuations/__init__.pyi
git commit -m "docs(finstack-py): lead module docstring with canonical MarketContext path

Module-level docstring opens with calibrate(envelope_json).market as the
single canonical path to build a MarketContext from quotes, framed as a
working example. Documents the two-track envelope structure (bootstrapping
via plan.steps; snapshot data via initial_market). The calibrate function's
docstring is expanded with the same structure plus diagnostic-checking
guidance (always verify .success and .rmse before consuming .market)."
```

---

## Task 9: Documentation — WASM `index.d.ts` lead

**Goal:** Add a module-level lead comment at the top of `finstack-wasm/index.d.ts` introducing the canonical path. (Type signatures were added in Task 1; this task only adds the narrative lead.)

**Files:**
- Modify: `finstack-wasm/index.d.ts:1-3`

- [ ] **Step 1: Replace the file's opening comment**

The current opening is:

```ts
// Type declarations for the finstack-wasm namespaced facade.
// Shapes follow `wasm-bindgen` JS names in `src/api/**` (see Rust `js_name`).
```

Replace with:

```ts
// Type declarations for the finstack-wasm namespaced facade.
// Shapes follow `wasm-bindgen` JS names in `src/api/**` (see Rust `js_name`).
//
// Building a MarketContext from quotes (canonical path):
//
//   import { valuations } from 'finstack-wasm/exports/valuations.js';
//   const envelopeJson = JSON.stringify({
//     schema: 'finstack.calibration',
//     plan: { id: 'usd_curves', quote_sets: {...}, steps: [...], settings: {} },
//     initial_market: null,
//   });
//   const resultJson = valuations.calibrate(envelopeJson);
//   const result = JSON.parse(resultJson);  // CalibrationResultEnvelope
//   const marketJson = JSON.stringify(result.result.final_market);
//
// `result.result.final_market` is the materialized MarketContextState ready
// for any downstream pricing / scenario / attribution call that takes a
// market_json argument. Always check the per-step report
// (`result.result.step_reports`) and the plan summary
// (`result.result.report`) to confirm the curves actually fit before using
// the market downstream.
//
// `validateCalibrationJson` is a fast pre-flight check that canonicalizes
// the envelope without solving — use it to surface schema errors early.
```

- [ ] **Step 2: Verify the file is still valid TypeScript**

Run: `npm --prefix finstack-wasm run typecheck` (if available) or visually inspect that comments are well-formed.
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add finstack-wasm/index.d.ts
git commit -m "docs(wasm): lead index.d.ts with canonical MarketContext path

Adds a top-of-file comment introducing valuations.calibrate as the
canonical path to build a MarketContext from quotes via JS, with a
working JSON envelope example and the standard 'check the report
before using the market' guidance."
```

---

## Task 10: Python notebook scaffold

**Goal:** Ship a single-cell end-to-end notebook demonstrating the canonical Python flow: read envelope JSON → `calibrate` → check residuals → query the resulting `MarketContext`. This is the discoverable analyst entry point. Phase 2 expands the notebook into a fuller walkthrough.

**Files:**
- Create: `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb`

- [ ] **Step 1: Create the notebook file**

The simplest path is to author the notebook interactively in Jupyter and save it. If creating from a script is preferred, use `nbformat` (Python). The notebook should contain at minimum:

**Cell 1 (markdown):**

```markdown
# Build a `MarketContext` from raw quotes — canonical path tour

This notebook walks through the analyst-facing flow for building a
`MarketContext` from a JSON `CalibrationEnvelope`. It is the canonical
entry point — `calibrate(envelope_json).market` — and the same surface is
exposed in Rust (`finstack_valuations::calibration::api::engine::execute`)
and JavaScript (`valuations.calibrate(envelopeJson)`).

We load one of the reference envelopes shipped under
`finstack/valuations/examples/market_bootstrap/`, run it through the
calibration engine, verify residuals, and access the resulting context.
```

**Cell 2 (code):**

```python
import json
from pathlib import Path

from finstack.valuations import calibrate

# Path to the reference envelope. Adjust if running outside the repo root.
REPO_ROOT = Path.cwd()
while not (REPO_ROOT / "finstack" / "valuations" / "examples" / "market_bootstrap").exists():
    if REPO_ROOT == REPO_ROOT.parent:
        raise RuntimeError("could not locate the repo root from cwd")
    REPO_ROOT = REPO_ROOT.parent

envelope_path = (
    REPO_ROOT
    / "finstack"
    / "valuations"
    / "examples"
    / "market_bootstrap"
    / "01_usd_discount.json"
)
envelope_json = envelope_path.read_text()
envelope = json.loads(envelope_json)
print(f"schema: {envelope['schema']}")
print(f"plan id: {envelope['plan']['id']}")
print(f"steps: {[step['id'] for step in envelope['plan']['steps']]}")
```

**Cell 3 (code):**

```python
result = calibrate(envelope_json)
print(f"success: {result.success}")
print(f"rmse: {result.rmse:.3e}")
print(f"max residual: {result.max_residual:.3e}")
print(f"iterations: {result.iterations}")
result.report_to_dataframe()
```

**Cell 4 (code):**

```python
# `result.market` is the calibrated MarketContext, ready for downstream use.
market = result.market

# Look up the discount curve we just built.
curve = market.get_discount("USD-OIS")
print(f"discount factor at t=0:  {curve.df(0.0):.6f}")
print(f"discount factor at t=1y: {curve.df(1.0):.6f}")
print(f"discount factor at t=5y: {curve.df(5.0):.6f}")
```

**Cell 5 (markdown):**

```markdown
## Persisting the materialized market

`result.market_json` returns the same context as a JSON snapshot. This is
useful for caching a calibrated market between processes or for diff'ing
two calibrated states.

`MarketContext` round-trips through this snapshot losslessly:
deserialize via `MarketContext.from_json(...)` (Python) or
`MarketContext::try_from(state)` (Rust).
```

**Cell 6 (code):**

```python
# Round-trip via the materialized JSON snapshot.
snapshot_json = result.market_json
print(f"snapshot length: {len(snapshot_json)} bytes")
# Phase 2 of the notebook tour will demonstrate composing markets and
# pulling FX cross rates / bond prices / equity spots from initial_market.
```

If the notebook cannot be authored interactively, use the `nbformat` template:

```python
import nbformat as nbf

nb = nbf.v4.new_notebook()
nb.cells = [
    nbf.v4.new_markdown_cell(...),
    nbf.v4.new_code_cell(...),
    # ... 4 more cells
]
nbf.write(nb, "finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb")
```

The exact JSON structure must match Jupyter v4 notebook format. Avoid hand-writing the `.ipynb` JSON unless you've verified the format.

- [ ] **Step 2: Verify the notebook executes end-to-end**

Run: `uv run jupyter nbconvert --to notebook --execute finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb --output executed_market_bootstrap_tour.ipynb`
Expected: All cells execute without error.

Delete the executed copy after verification (`rm finstack-py/examples/notebooks/01_foundations/executed_market_bootstrap_tour.ipynb`).

If `01_usd_discount.json` does not yet exist (Task 2 not complete), this step is unblocked once Task 2 lands.

- [ ] **Step 3: Commit**

```bash
git add finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb
git commit -m "docs(finstack-py): add market_bootstrap_tour notebook scaffold

Phase 1 notebook: end-to-end demonstration of the canonical
calibrate(envelope_json).market flow using the 01_usd_discount reference
envelope. Loads the JSON, calls calibrate, prints residuals and the
report dataframe, accesses the calibrated discount curve, and shows
round-trip via market_json. Phase 2 will expand the tour with composition
and snapshot-data examples (FX matrices, bond prices, equity spots)."
```

---

## Task 11: End-to-end verification & spec acceptance check

**Goal:** Run the full verification suite from the spec and confirm every Phase 1 acceptance criterion is satisfied.

- [ ] **Step 1: Run focused tests**

Run each of the following individually; all must pass:

```bash
cargo test -p finstack-valuations --test calibration reference_envelopes
cargo test -p finstack-valuations --test golden golden_pricing_fixtures_from_existing_json_files
cargo test -p finstack-wasm api::valuations::calibration
uv run pytest -v finstack-py/tests/golden/
npm --prefix finstack-wasm run test
```

- [ ] **Step 2: Run the full project verification stack**

```bash
mise run all-fmt
mise run all-lint
mise run python-build
mise run all-test
```

Expected: green across the board. Any new lint/format issues introduced in Tasks 1–10 must be fixed before declaring Phase 1 complete.

- [ ] **Step 3: Check Phase 1 acceptance criteria**

Walk through `docs/2026-05-08-market-bootstrap-phase-1-foundation-design.md` §7 and tick each:

- [ ] `calibrate` and `validateCalibrationJson` callable from JavaScript via the public WASM exports. (Task 1)
- [ ] `finstack/valuations/src/calibration/mod.rs` crate doc opens with the canonical-path narrative. (Task 7)
- [ ] `finstack-py/finstack/valuations/__init__.pyi` module docstring leads with `calibrate(envelope).market`. (Task 8)
- [ ] Three example envelope JSON files exist and parse as `CalibrationEnvelope`. (Tasks 2, 3, 4)
- [ ] Three Rust integration tests pass: each demonstrates the produced `MarketContext` answers a typical accessor query. (Tasks 2, 3, 4)
- [ ] Pricing golden runners (Rust + Python) accept `market_envelope`. Both-keys-present is rejected with the documented error message. (Tasks 5, 6)
- [ ] At least one synthetic pricing golden fixture uses `market_envelope` and passes. (Task 5)
- [ ] One Python notebook cell shows: read JSON file → `calibrate` → check residuals → query the resulting `MarketContext`. (Task 10)

If any criterion is unmet, return to the relevant task and finish the work before declaring Phase 1 complete.

- [ ] **Step 4: Final commit (if any unstaged cleanup)**

If the verification stack flagged formatter / lint adjustments in any of the touched files, stage and commit them as a final cleanup commit:

```bash
git status
git diff --stat
git add <fixed-files>
git commit -m "chore: format and lint cleanup for market bootstrap phase 1"
```

---

## Phase 1 done

When this plan is complete, `calibrate(envelope_json).market` is the documented, tested, reachable canonical path across Rust, Python, and JavaScript. Three reference envelopes demonstrate the two-track structure end-to-end. Pricing golden fixtures can be authored against `market_envelope` for any future work. The notebook scaffold is in place.

Phase 2 (reference-catalog completion + CDX IG 46 fixture migration) is the natural next slice. See [`docs/2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md`](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md).
