# Market Bootstrap Phase 2 — Reference Catalog Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Round out the reference envelope catalog from 3 → 12 examples, migrate the CDX IG 46 production pricing fixture to use `market_envelope`, and expand the analyst notebook with composition / snapshot-data / accessor sections.

**Architecture:** Each new envelope follows the Phase 1 pattern — JSON file under `finstack/valuations/examples/market_bootstrap/`, integration test appended to the existing `reference_envelopes.rs`, accessor assertion that documents how to query the produced `MarketContext`. The CDX migration replaces hand-entered knot points with quote-driven calibration steps. The notebook is expanded via `nbformat` to add three new sections.

**Tech Stack:** Rust (`finstack-valuations`), PyO3 (`finstack-py`), `nbformat` for notebook authoring, no new dependencies.

**Spec reference:** [`docs/2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md`](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md)

**Phase 1 plan reference:** [`docs/2026-05-08-market-bootstrap-phase-1-foundation-plan.md`](2026-05-08-market-bootstrap-phase-1-foundation-plan.md). Phase 1 ships the helpers (`examples_dir`, `load_envelope`, `execute`) in [`reference_envelopes.rs`](../finstack/valuations/tests/calibration/reference_envelopes.rs) marked `pub(crate)` for reuse here.

**Phase 1 learnings to apply throughout:**
1. `MarketContext::get_*` accessors take `impl AsRef<str>` — pass curve IDs as plain `&str`, no `.parse()` needed.
2. `HazardCurve::sp(t)` is the survival method (NOT `.survival(t)`).
3. `CalibrationMethod` has no `rename_all`, so its variant serializes as PascalCase `"Bootstrap"`.
4. `CalibrationEnvelope` uses `#[serde(deny_unknown_fields)]`; description must live at `plan.description`, not at envelope top level.
5. `FxMatrixState.quotes` is `Vec<(Currency, Currency, f64)>`, serializes as JSON tuple-arrays `["EUR", "USD", 1.0850]`, NOT object form.
6. After authoring an envelope JSON, run `cargo fmt -p finstack-valuations` before committing — Phase 1 hit a clippy/fmt issue mid-way that required a follow-up commit.
7. Standard Phase 1 base date: `"2026-05-08"`. Use this consistently across new envelopes.
8. For each new envelope: the safest path to a correct serde JSON is to write a temporary `#[test]` in the relevant in-tree calibration test file, construct the envelope programmatically, serialize via `serde_json::to_string_pretty`, copy the output to the example file, then DELETE the scratch test before committing.

**Commit policy:** Same as Phase 1 — no commits without explicit user approval. Each task ends with a `git commit` step shown for completeness; the implementer must confirm with the user before running it (or batch all commits at end if the user prefers).

---

## File Structure

### Files to create

| Path | Responsibility |
|---|---|
| `finstack/valuations/examples/market_bootstrap/02_usd_3m_forward_curve.json` | Track A — forward curve calibration step on top of an `initial_market` discount curve. |
| `finstack/valuations/examples/market_bootstrap/04_cdx_ig_hazard.json` | Track A — CDX index hazard curve calibration with IMM-date conventions. |
| `finstack/valuations/examples/market_bootstrap/05_cdx_base_correlation.json` | Track A — CDX tranche base correlation calibration. |
| `finstack/valuations/examples/market_bootstrap/06_cdx_index_vol.json` | Track A — SABR/credit vol surface for CDX index options. |
| `finstack/valuations/examples/market_bootstrap/07_swaption_vol_surface.json` | Track A — swaption vol surface calibration. |
| `finstack/valuations/examples/market_bootstrap/08_equity_vol_surface.json` | Track A — SABR/equity vol surface calibration. |
| `finstack/valuations/examples/market_bootstrap/10_bond_prices.json` | Track B — bond prices supplied via `initial_market.prices`. |
| `finstack/valuations/examples/market_bootstrap/11_equity_spots_dividends.json` | Track B — equity spots in `prices` + dividend schedules in `initial_market.dividends`. |
| `finstack/valuations/examples/market_bootstrap/12_full_credit_desk_market.json` | Composite Track A chained — discount → hazard → base correlation, with FX in `initial_market`. |

### Files to modify

| Path | Change |
|---|---|
| `finstack/valuations/tests/calibration/reference_envelopes.rs` | Append nine new integration tests, one per new envelope. Reuse existing `pub(crate)` helpers. |
| `finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json` | Replace embedded `market` snapshot with `market_envelope` carrying quote-driven discount + hazard steps. Recompute tolerances. |
| `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb` | Append three new sections (Build / Compose / Snapshot-only) plus a Phase-4 placeholder. |

---

## Task 1: Reference envelope #02 — USD 3M forward curve

**Goal:** Demonstrate a forward step calibrated on top of an `initial_market` discount curve.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/02_usd_3m_forward_curve.json`

- [ ] **Step 1: Append the failing test**

In `finstack/valuations/tests/calibration/reference_envelopes.rs`, after the existing `example_09_*` test, add:

```rust
#[test]
fn example_02_usd_3m_forward_builds_queryable_curve() {
    let envelope = load_envelope("02_usd_3m_forward_curve.json");
    let market = execute(&envelope);

    // Discount curve passes through unchanged from initial_market.
    market
        .get_discount("USD-OIS")
        .expect("discount curve carried through from initial_market");

    // Forward curve must be produced by the calibration step.
    let forward = market
        .get_forward("USD-SOFR-3M")
        .expect("forward curve present after forward step");

    // Forward rate at t=1y should be a sane positive rate (i.e., calibration
    // produced non-zero knots).
    let rate_one_year = forward.rate(1.0);
    assert!(
        rate_one_year > 0.0 && rate_one_year < 0.20,
        "forward rate at t=1y should be in (0, 0.20), got {rate_one_year}"
    );
}
```

If the forward accessor is not exactly `market.get_forward(...)` or the curve type's rate method has a different name, adjust to match `finstack/core/src/market_data/context/getters.rs` and the `ForwardCurve` impl. The test's *intent* is "forward curve present and produces a positive rate" — keep that, fix the call shape.

- [ ] **Step 2: Run the failing test (file does not yet exist)**

Run: `cargo test -p finstack-valuations --test calibration example_02_usd_3m_forward_builds_queryable_curve`
Expected: FAIL with "No such file or directory".

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/02_usd_3m_forward_curve.json`. Required shape:

1. `"schema": "finstack.calibration"`.
2. `plan.description` explaining the envelope.
3. `plan.id` = `"usd_sofr_3m_forward"`.
4. `plan.quote_sets` contains one set (e.g., `"sofr_3m_quotes"`) with 4–6 `RateQuote` entries (mix of futures and FRAs) that bootstrap a 3M forward curve.
5. `plan.steps` contains exactly one `forward` step with `id` = `"USD-SOFR-3M"`, `discount_curve_id` = `"USD-OIS"`, `base_date` = `"2026-05-08"`, and project-default conventions for the underlying tenor.
6. `initial_market` carries a single `USD-OIS` discount curve in `MarketContextState` form (mirror Phase 1's `03_single_name_hazard.json:124-143` for the curve shape; reuse those exp(-0.05*t) DFs for consistency).

To produce the JSON, write a scratch `#[test]` in `finstack/valuations/tests/calibration/quote_construction.rs` (or wherever an existing forward-step test constructs a working envelope). Build the envelope programmatically, run `serde_json::to_string_pretty(&envelope)` to a file, copy the file content into the example, and DELETE the scratch test before committing.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_02_usd_3m_forward_builds_queryable_curve`
Expected: PASS.

If the engine fails to converge, adjust the rate quotes to be self-consistent (monotonically reasonable rates, plausible forward shape).

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/02_usd_3m_forward_curve.json
git commit -m "test(valuations): add 02_usd_3m_forward_curve reference envelope

Phase 2 reference envelope: forward curve calibration step (USD-SOFR-3M)
on top of an initial_market discount curve (USD-OIS). Demonstrates the
common rates pattern of bootstrapping forward curves against a fixed
discount-curve assumption. Asserts the produced forward curve answers
a typical rate-lookup query."
```

---

## Task 2: Reference envelope #04 — CDX IG hazard curve

**Goal:** Demonstrate a hazard step with index conventions (IMM dates, recovery, doc clauses).

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/04_cdx_ig_hazard.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_04_cdx_ig_hazard_builds_queryable_curve() {
    let envelope = load_envelope("04_cdx_ig_hazard.json");
    let market = execute(&envelope);

    market
        .get_discount("USD-OIS")
        .expect("discount carried through from initial_market");

    // CDX index hazard curve must be present.
    let hazard = market
        .get_hazard("CDX-NA-IG-46")
        .expect("CDX hazard curve present after calibration");

    let survival_5y = hazard.sp(5.0);
    assert!(
        survival_5y > 0.0 && survival_5y < 1.0,
        "5y survival should be in (0, 1), got {survival_5y}"
    );
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_04_cdx_ig_hazard_builds_queryable_curve`
Expected: FAIL (file missing).

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/04_cdx_ig_hazard.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description` explaining the envelope.
3. `plan.id` = `"cdx_ig_46_hazard"`.
4. `plan.quote_sets` with one set of 5 `CdsQuote::CdsParSpread` (or whichever CDS-quote variant the hazard step expects for index conventions) at IMM tenors (1Y, 3Y, 5Y, 7Y, 10Y). Choose plausible CDX-IG spreads (~50–110 bp).
5. `plan.steps` contains one `hazard` step with `id` = `"CDX-NA-IG-46"`, `discount_curve_id` = `"USD-OIS"`, `recovery_rate` = `0.4`, `entity` reflecting the index, `currency` = `"USD"`, IMM dates enabled, and `doc_clause` matching CDX standard conventions.
6. `initial_market` carries the same `USD-OIS` discount curve as Phase 1's `03_single_name_hazard.json` (use the exp(-0.05*t) DFs).

For the index-specific fields (IMM dates, doc clauses, index conventions), reference `finstack/valuations/tests/calibration/hazard_curve.rs` or any in-tree test that exercises CDX index hazard calibration. Author programmatically and dump JSON.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_04_cdx_ig_hazard_builds_queryable_curve`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/04_cdx_ig_hazard.json
git commit -m "test(valuations): add 04_cdx_ig_hazard reference envelope

Phase 2 reference envelope: CDX.NA.IG.46 index hazard curve calibration
with IMM-date conventions and recovery_rate 0.4 (ISDA standard for
senior unsecured). Differs from 03_single_name_hazard by using index
conventions instead of single-name issuer terms; the hazard step's
entity, currency, and doc_clause fields reflect this. Used as
initial_market input by 05 (base correlation) and 06 (CDX index vol)."
```

---

## Task 3: Reference envelope #05 — CDX base correlation

**Goal:** Demonstrate a base_correlation step calibrated against tranche quotes, depending on a CDX index hazard curve in `initial_market`.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/05_cdx_base_correlation.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_05_cdx_base_correlation_builds_queryable_curve() {
    let envelope = load_envelope("05_cdx_base_correlation.json");
    let market = execute(&envelope);

    market
        .get_discount("USD-OIS")
        .expect("discount carried through from initial_market");
    market
        .get_hazard("CDX-NA-IG-46")
        .expect("CDX index hazard carried through from initial_market");

    // Base correlation curve must be produced by the step.
    let bc = market
        .get_base_correlation("CDX-NA-IG-46-BASE-CORR")
        .expect("base correlation curve present after calibration");

    // Detachment-point lookup at 7% should be a correlation in [0, 1].
    let corr_7pct = bc.correlation(0.07);
    assert!(
        (0.0..=1.0).contains(&corr_7pct),
        "base correlation at 7% detachment should be in [0, 1], got {corr_7pct}"
    );
}
```

If the base-correlation accessor name differs (e.g., `get_correlation_curve`, `correlation_at`, etc.), adjust to the actual `MarketContext` API. The test's intent is "base correlation surface present and queryable at a detachment point."

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_05_cdx_base_correlation_builds_queryable_curve`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/05_cdx_base_correlation.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description` explaining what the envelope produces.
3. `plan.id` = `"cdx_ig_46_base_correlation"`.
4. `plan.quote_sets` contains one set of 4–5 `CDSTrancheQuote` entries at standard CDX detachment points (`0.03`, `0.07`, `0.10`, `0.15`, `0.30` is conventional for IG).
5. `plan.steps` contains one `base_correlation` step with `id` = `"CDX-NA-IG-46-BASE-CORR"`, `index_id` = `"CDX-NA-IG-46"` (referencing the index hazard curve in `initial_market`), `series` = `46`, `maturity_years` = `5.0`, `base_date` = `"2026-05-08"`, `discount_curve_id` = `"USD-OIS"`, `currency` = `"USD"`, `detachment_points` matching the quotes, and `use_imm_dates` = `true`.
6. `initial_market` carries:
   - `USD-OIS` discount curve (same as Phase 1).
   - `CDX-NA-IG-46` hazard curve (snapshot — copy from a programmatically-built hazard with knot points covering 1Y/3Y/5Y/7Y/10Y).

To produce the JSON: scratch `#[test]` in `finstack/valuations/tests/calibration/base_correlation.rs`, programmatic construction, dump, copy.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_05_cdx_base_correlation_builds_queryable_curve`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/05_cdx_base_correlation.json
git commit -m "test(valuations): add 05_cdx_base_correlation reference envelope

Phase 2 reference envelope: base correlation curve for CDX.NA.IG.46
calibrated from tranche quotes against the index hazard curve in
initial_market. Demonstrates layered composition (discount + hazard
in initial_market, base correlation as the calibrated step). Asserts
the resulting correlation surface answers a typical detachment-point
lookup."
```

---

## Task 4: Reference envelope #06 — CDX index vol surface

**Goal:** Demonstrate a SABR/credit vol surface calibrated for CDX index options.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/06_cdx_index_vol.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_06_cdx_index_vol_builds_queryable_surface() {
    let envelope = load_envelope("06_cdx_index_vol.json");
    let market = execute(&envelope);

    market
        .get_discount("USD-OIS")
        .expect("discount carried through from initial_market");
    market
        .get_hazard("CDX-NA-IG-46")
        .expect("CDX index hazard carried through from initial_market");

    let surface = market
        .get_vol_surface("CDX-NA-IG-46-CDSO-VOL")
        .expect("CDX index vol surface present after calibration");

    // ATM vol at the calibrated expiry should be in a sane range.
    // Surface accessors vary; use whichever method the surface type exposes
    // for "implied vol at (expiry, strike)". If the SABR-credit surface has
    // a different accessor name, adapt to it.
    let _ = surface; // sanity: surface handle was returned
}
```

If the vol surface exposes a specific accessor (e.g., `vol(expiry, strike)`), tighten the assertion to a numeric check (vol in (0, 2)). For Phase 2's purpose, asserting the surface is *present* in the calibrated context is sufficient — the surface API itself is exercised by domain tests elsewhere.

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_06_cdx_index_vol_builds_queryable_surface`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/06_cdx_index_vol.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description` (informative).
3. `plan.id` = `"cdx_ig_46_cdso_vol"`.
4. `plan.quote_sets` contains one set of `VolQuote` entries at a small grid of (expiry, strike) points appropriate for CDX index options.
5. `plan.steps` contains one `vol_surface` step with `id` = `"CDX-NA-IG-46-CDSO-VOL"`, asset class = credit (or however the schema distinguishes credit vol from equity vol), `index_id` = `"CDX-NA-IG-46"`, `discount_curve_id` = `"USD-OIS"`, `base_date` = `"2026-05-08"`. Use SABR with rates-style defaults (β fixed, calibrate α/ν/ρ).
6. `initial_market` carries:
   - `USD-OIS` discount curve.
   - `CDX-NA-IG-46` hazard curve (snapshot copied from earlier).

Reference the existing `tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json` `surfaces` block for the exact SABR vol-surface state shape, but build the calibration step input programmatically by reading `finstack/valuations/tests/calibration/swaption_vol.rs` or similar. **The reviewer's `Task 7` warning applies**: if the equity SABR vs credit SABR distinction lives in the step params, don't blindly copy a swaption-vol step.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_06_cdx_index_vol_builds_queryable_surface`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/06_cdx_index_vol.json
git commit -m "test(valuations): add 06_cdx_index_vol reference envelope

Phase 2 reference envelope: SABR vol surface for CDX.NA.IG.46 index
options, calibrated from a (expiry, strike) grid of VolQuotes against
the index hazard curve in initial_market. Demonstrates how analysts
build credit-derivative vol surfaces alongside the underlying credit
curve."
```

---

## Task 5: Reference envelope #07 — Swaption vol surface

**Goal:** Demonstrate a swaption_vol step calibrated against a swaption normal-vol grid, layered on discount + forward curves.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/07_swaption_vol_surface.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_07_swaption_vol_surface_builds_queryable_surface() {
    let envelope = load_envelope("07_swaption_vol_surface.json");
    let market = execute(&envelope);

    market
        .get_discount("USD-OIS")
        .expect("discount carried through from initial_market");
    market
        .get_forward("USD-SOFR-3M")
        .expect("forward carried through from initial_market");

    let surface = market
        .get_vol_surface("USD-SWAPTION-NORMAL-VOL")
        .expect("swaption vol surface present after calibration");

    let _ = surface;
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_07_swaption_vol_surface_builds_queryable_surface`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/07_swaption_vol_surface.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description`.
3. `plan.id` = `"usd_swaption_normal_vol"`.
4. `plan.quote_sets` contains one set of `VolQuote::ShiftedLognormal` (or `Normal`, whichever the schema's swaption-vol variant is) entries at a small (expiry, swap-tenor, strike) grid. Use a 3×3 grid (e.g., expiries 1Y/5Y/10Y × tenors 2Y/5Y/10Y, ATM strike) at plausible normal vols (50–80 bp).
5. `plan.steps` contains one `swaption_vol` step with `id` = `"USD-SWAPTION-NORMAL-VOL"`, `discount_curve_id` = `"USD-OIS"`, `forward_curve_id` = `"USD-SOFR-3M"`, `base_date` = `"2026-05-08"`, and the appropriate SABR-grid interpolation defaults.
6. `initial_market` carries:
   - `USD-OIS` discount curve.
   - `USD-SOFR-3M` forward curve (snapshot — reuse the curve produced by Task 1's example, but as a materialized state).

Reference `finstack/valuations/tests/calibration/swaption_vol.rs` for the `swaption_vol` step's exact param shape.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_07_swaption_vol_surface_builds_queryable_surface`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/07_swaption_vol_surface.json
git commit -m "test(valuations): add 07_swaption_vol_surface reference envelope

Phase 2 reference envelope: USD swaption normal-vol surface calibrated
against a 3x3 (expiry, tenor) grid of VolQuotes, layered on discount +
forward curves in initial_market. Demonstrates the standard rates-vol
analyst workflow."
```

---

## Task 6: Reference envelope #08 — Equity vol surface

**Goal:** Demonstrate a SABR/equity vol surface calibrated against equity-option vol quotes, with discount + dividend schedule + spot in `initial_market`.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/08_equity_vol_surface.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_08_equity_vol_surface_builds_queryable_surface() {
    let envelope = load_envelope("08_equity_vol_surface.json");
    let market = execute(&envelope);

    market
        .get_discount("USD-OIS")
        .expect("discount carried through from initial_market");

    // Equity spot price must be present in initial_market.prices.
    let spot = market
        .get_price("AAPL")
        .expect("equity spot price present in initial_market.prices");
    assert!(spot > 0.0, "AAPL spot should be positive, got {spot}");

    let surface = market
        .get_vol_surface("AAPL-EQUITY-VOL")
        .expect("equity vol surface present after calibration");

    let _ = surface;
}
```

If `get_price` returns a `MarketScalar` rather than a raw `f64`, adjust the assertion (e.g., `spot.amount() > 0.0`).

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_08_equity_vol_surface_builds_queryable_surface`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/08_equity_vol_surface.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description`.
3. `plan.id` = `"aapl_equity_vol"`.
4. `plan.quote_sets` with `VolQuote` entries at a small (expiry, strike) grid for AAPL options. Use plausible equity vols (15–35%).
5. `plan.steps` contains one `vol_surface` step with `id` = `"AAPL-EQUITY-VOL"`, asset class = equity, `underlier` = `"AAPL"`, `discount_curve_id` = `"USD-OIS"`, `base_date` = `"2026-05-08"`. Use SABR with equity defaults (β=1, calibrate α/ν/ρ).
6. `initial_market` carries:
   - `USD-OIS` discount curve.
   - `prices`: `{"AAPL": <spot scalar>}` (e.g., 175.0 or whatever the schema accepts).
   - `dividends`: a single AAPL dividend schedule (small list of expected dividend dates + amounts; reference `finstack/core/src/market_data/term_structures/` or an existing fixture for the exact `DividendSchedule` shape).

The dividend schedule shape is non-obvious; the implementer should examine existing usage. If there's no in-tree test that calibrates equity SABR with dividends, this is the first one — extra care needed. The reviewer's spec note (§8 risks): "consider whether to ship the dependent state inline or via a `composite` chained plan that builds dividend/spot in earlier steps. Decide during implementation; both work." — for Phase 2, keep it inline in `initial_market` for simplicity.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_08_equity_vol_surface_builds_queryable_surface`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/08_equity_vol_surface.json
git commit -m "test(valuations): add 08_equity_vol_surface reference envelope

Phase 2 reference envelope: AAPL equity SABR vol surface calibrated
against an (expiry, strike) grid of VolQuotes, with discount curve +
dividend schedule + equity spot supplied via initial_market. The most
dependency-rich Phase 2 envelope; demonstrates how Track A and Track B
data combine in a single envelope."
```

---

## Task 7: Reference envelope #10 — Bond prices (Track B)

**Goal:** Demonstrate Track B for fixed-income: bond prices supplied via `initial_market.prices`, no calibration steps.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/10_bond_prices.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_10_bond_prices_supports_lookup() {
    let envelope = load_envelope("10_bond_prices.json");
    let market = execute(&envelope);

    // At least two distinct bond IDs must be retrievable from prices.
    let p1 = market
        .get_price("US-TREASURY-10Y-2026-05-08")
        .expect("US 10Y treasury price present in initial_market.prices");
    let p2 = market
        .get_price("IBM-7YR-2033")
        .expect("IBM corporate bond price present");

    assert!(p1 > 0.0, "treasury price should be positive, got {p1}");
    assert!(p2 > 0.0, "IBM bond price should be positive, got {p2}");
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_10_bond_prices_supports_lookup`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/10_bond_prices.json`:

```json
{
  "schema": "finstack.calibration",
  "plan": {
    "id": "bond_prices_snapshot",
    "description": "Snapshot-only example: bond clean prices supplied via initial_market.prices, with no calibration steps. Demonstrates Track B for fixed-income — non-bootstrapped bond marks loaded as scalars keyed by bond ID. The MarketScalar shape is shared with equity spot prices (see 11_equity_spots_dividends.json).",
    "quote_sets": {},
    "steps": [],
    "settings": {}
  },
  "initial_market": {
    "version": 2,
    "curves": [],
    "fx": null,
    "surfaces": [],
    "prices": {
      "US-TREASURY-10Y-2026-05-08": 99.875,
      "IBM-7YR-2033": 102.50,
      "MSFT-5YR-2031": 101.125
    },
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

**Verify** the `prices` serde shape against `finstack/core/src/market_data/context/state_serde.rs` — the values may be required as `MarketScalar` objects rather than raw floats. If so, adjust the JSON to use `{"amount": 99.875, "currency": "USD"}` (or whatever the actual `MarketScalar` serialization is). The Phase 1 `usd_deposit_3m.json` fixture has `"prices": {}` (empty), so it doesn't reveal the populated shape — the implementer must look at the type definition or a richer in-tree fixture.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_10_bond_prices_supports_lookup`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/10_bond_prices.json
git commit -m "test(valuations): add 10_bond_prices reference envelope

Phase 2 reference envelope: empty calibration plan, three bond clean
prices supplied via initial_market.prices keyed by bond ID. Track B
example for fixed-income marks. Asserts at least two distinct bond
IDs are retrievable through market.get_price."
```

---

## Task 8: Reference envelope #11 — Equity spots + dividends (Track B)

**Goal:** Demonstrate Track B for equities: spot prices in `initial_market.prices`, dividend schedules in `initial_market.dividends`.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/11_equity_spots_dividends.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_11_equity_spots_and_dividends_support_lookup() {
    let envelope = load_envelope("11_equity_spots_dividends.json");
    let market = execute(&envelope);

    let aapl_spot = market
        .get_price("AAPL")
        .expect("AAPL spot present in initial_market.prices");
    assert!(aapl_spot > 0.0, "AAPL spot should be positive, got {aapl_spot}");

    let msft_spot = market
        .get_price("MSFT")
        .expect("MSFT spot present");
    assert!(msft_spot > 0.0, "MSFT spot should be positive, got {msft_spot}");

    // Dividend schedule for AAPL must be retrievable and non-empty.
    let aapl_divs = market
        .get_dividends("AAPL")
        .expect("AAPL dividend schedule present in initial_market.dividends");
    assert!(
        !aapl_divs.is_empty(),
        "AAPL dividend schedule should be non-empty"
    );
}
```

If the dividends accessor is not `market.get_dividends(...)`, adjust to match the actual API (`finstack/core/src/market_data/context/getters.rs`). Verify `aapl_divs` returns something with an `is_empty()` method or a usable iterator.

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_11_equity_spots_and_dividends_support_lookup`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/11_equity_spots_dividends.json`. Required shape:

1. `"schema": "finstack.calibration"`.
2. `plan.description` informative.
3. `plan.id` = `"equity_spots_dividends_snapshot"`.
4. `plan.quote_sets` empty, `plan.steps` empty (Track B).
5. `initial_market.prices` with two equity spots: `"AAPL"` and `"MSFT"` (use plausible spot values, e.g., 175.0 and 410.0). Match the `MarketScalar` serde shape from Task 7.
6. `initial_market.dividends` with at least one populated `DividendSchedule` (for AAPL): a small list of (ex-date, amount) entries spanning the next year. The exact `DividendSchedule` serde shape comes from `finstack/core/src/market_data/term_structures/dividend.rs` (or wherever the type lives). Verify by reading the type's `#[derive(Deserialize)]` and any in-tree fixture that uses it.

To produce: the safest approach is again to write a scratch test that constructs `DividendSchedule` programmatically, dump JSON, copy. If no in-tree fixture exists for dividends, this is the first user-facing one — careful with field names.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_11_equity_spots_and_dividends_support_lookup`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/11_equity_spots_dividends.json
git commit -m "test(valuations): add 11_equity_spots_dividends reference envelope

Phase 2 reference envelope: empty calibration plan, two equity spot
prices in initial_market.prices and an AAPL dividend schedule in
initial_market.dividends. Track B example completing the
fixed-income / equity / FX trio of snapshot-data tracks."
```

---

## Task 9: Reference envelope #12 — Full credit-desk composite

**Goal:** Demonstrate a chained Track A plan: discount → hazard → base correlation, with FX in `initial_market`. The most ambitious Phase 2 envelope and the canonical "what a credit analyst's morning bootstrap looks like" example.

**Files:**
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs` (append a test)
- Create: `finstack/valuations/examples/market_bootstrap/12_full_credit_desk_market.json`

- [ ] **Step 1: Append the failing test**

```rust
#[test]
fn example_12_full_credit_desk_market_chains_steps() {
    let envelope = load_envelope("12_full_credit_desk_market.json");
    let market = execute(&envelope);

    // All three calibrated curves should be present.
    market
        .get_discount("USD-OIS")
        .expect("discount curve produced by step");
    let hazard = market
        .get_hazard("CDX-NA-IG-46")
        .expect("hazard curve produced by step");
    market
        .get_base_correlation("CDX-NA-IG-46-BASE-CORR")
        .expect("base correlation curve produced by step");

    // FX matrix from initial_market must survive.
    let fx = market.fx().expect("fx matrix from initial_market");
    let _ = fx;

    // Sanity: hazard at 5y is in (0, 1).
    let sp_5y = hazard.sp(5.0);
    assert!(
        sp_5y > 0.0 && sp_5y < 1.0,
        "5y survival should be in (0, 1), got {sp_5y}"
    );
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p finstack-valuations --test calibration example_12_full_credit_desk_market_chains_steps`
Expected: FAIL.

- [ ] **Step 3: Author the envelope JSON**

Create `finstack/valuations/examples/market_bootstrap/12_full_credit_desk_market.json`:

1. `"schema": "finstack.calibration"`.
2. `plan.description`: explain the chained-plan pattern and what an analyst gets at the end.
3. `plan.id` = `"full_credit_desk_market"`.
4. `plan.quote_sets` contains three named sets:
   - `usd_rate_quotes` — 4–6 deposit + IRS quotes (mirror Phase 1's `01_usd_discount.json`).
   - `cdx_ig_46_cds_quotes` — 5 CDS index quotes (mirror Task 2's `04_cdx_ig_hazard.json`).
   - `cdx_ig_46_tranche_quotes` — 4–5 tranche quotes (mirror Task 3's `05_cdx_base_correlation.json`).
5. `plan.steps` contains three steps **in dependency order**:
   1. `discount` step producing `USD-OIS` from `usd_rate_quotes`.
   2. `hazard` step producing `CDX-NA-IG-46` from `cdx_ig_46_cds_quotes` (depends on `USD-OIS`).
   3. `base_correlation` step producing `CDX-NA-IG-46-BASE-CORR` from `cdx_ig_46_tranche_quotes` (depends on `USD-OIS` and `CDX-NA-IG-46`).
6. `initial_market.fx` carries 3 cross rates (e.g., EUR/USD, USD/JPY, GBP/USD), matching Phase 1's `09_fx_matrix.json` style. Other `initial_market` fields empty.

The plan engine must execute the three steps in dependency order. If the engine doesn't auto-topo-sort, declare them in the JSON order shown above.

- [ ] **Step 4: Run cargo fmt and the test**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo test -p finstack-valuations --test calibration example_12_full_credit_desk_market_chains_steps`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/tests/calibration/reference_envelopes.rs \
        finstack/valuations/examples/market_bootstrap/12_full_credit_desk_market.json
git commit -m "test(valuations): add 12_full_credit_desk_market composite envelope

Phase 2 reference envelope: chained discount → hazard → base correlation
plan with FX matrix in initial_market. Demonstrates the canonical
'analyst's morning bootstrap' for a credit desk — one envelope produces
the full set of credit-pricing curves plus FX, in a single calibrate()
call. Asserts all three calibrated curves and the snapshot FX matrix
are present in the resulting MarketContext."
```

---

## Task 10: CDX IG 46 fixture migration to `market_envelope`

**Goal:** Replace the embedded materialized `market` block in the CDX IG 46 CDS option pricing fixture with a `market_envelope` carrying quote-driven discount + hazard calibration steps.

**Files:**
- Modify: `finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json`

- [ ] **Step 1: Inspect the current fixture**

Run: `cat finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json | head -50` (and similar tail commands) to understand the current structure. Identify:
- The S531 deposit + swap rates currently embedded in `inputs.market.curves[type=discount].knot_points` (along with the day-count + interpolation conventions).
- The CDX par spreads currently embedded in `inputs.market.curves[type=hazard].knot_points` (along with recovery, IMM convention, currency).
- The SABR vol surface in `inputs.market.surfaces` (KEEP this in `initial_market.surfaces`; only the rates/credit curves move to calibration steps).
- The `valuation_date`, `expected_outputs`, and existing `tolerances`.

Per the spec (§5.2): the SABR vol surface stays in `initial_market.surfaces` for now (deferred to a later sweep that calibrates from raw vol quotes).

- [ ] **Step 2: Build the migrated `market_envelope` block**

Replace `inputs.market` with `inputs.market_envelope`. Structure:

```json
"market_envelope": {
  "schema": "finstack.calibration",
  "plan": {
    "id": "cdx_ig_46_pricing_market",
    "description": "Bootstraps the USD-OIS discount and CDX.NA.IG.46 hazard curves from S531 swap-curve quotes and CDX par spreads. The SABR vol surface is supplied as a snapshot in initial_market.surfaces (calibration of the vol surface from raw option quotes is deferred).",
    "quote_sets": {
      "s531_rates": [ /* deposit + swap quotes from the original discount knots */ ],
      "cdx_ig_46_cds": [ /* CDX par spreads from the original hazard knots */ ]
    },
    "steps": [
      {
        "id": "USD-S531-SWAP-2026-05-07",
        "quote_set": "s531_rates",
        "kind": "discount",
        "currency": "USD",
        "base_date": "<existing valuation_date>",
        "method": "Bootstrap",
        ...
      },
      {
        "id": "CDX-NA-IG-46-CBBT",
        "quote_set": "cdx_ig_46_cds",
        "kind": "hazard",
        "discount_curve_id": "USD-S531-SWAP-2026-05-07",
        "recovery_rate": 0.4,
        "base_date": "<existing valuation_date>",
        ...
      }
    ],
    "settings": {}
  },
  "initial_market": {
    "version": 2,
    "curves": [],
    "fx": <existing fx block>,
    "surfaces": [ /* the existing SABR vol surface unchanged */ ],
    "prices": {},
    "series": [],
    "inflation_indices": [],
    "dividends": [],
    "credit_indices": [ /* if the original fixture had credit_indices, keep them */ ],
    "fx_delta_vol_surfaces": [],
    "vol_cubes": [],
    "collateral": {}
  }
}
```

The exact discount-step and hazard-step parameter shapes must match the schema. Use Task 1 (`02_usd_3m_forward_curve.json`) and Task 2 (`04_cdx_ig_hazard.json`) as templates. The S531 swap-curve conventions (day count, interpolation, currency) should match what's in the existing fixture's discount curve. The CDX hazard conventions must match what's in the existing fixture's hazard curve.

- [ ] **Step 3: Run the golden suite**

Run: `cargo test -p finstack-valuations --test golden cds_option`
Run: `uv run pytest -v finstack-py/tests/golden/test_pricing_cds_option.py`

Expected outcome on first run: the test will likely *fail* with new actual values that differ from the original `actual_outputs_under_bloomberg_quadrature` and the existing `tolerances`. This is expected — the bootstrapped curves are not byte-identical to the hand-entered knots.

- [ ] **Step 4: Recompute residuals and update fixture metadata**

Capture the new actual outputs from the failing test run. Update:
- `actual_outputs_under_bloomberg_quadrature` — replace each metric with the new bootstrapped value.
- `tolerances` — widen/adjust as needed so the test passes against `expected_outputs` (the raw Bloomberg screen values, which DO NOT change).
- `provenance.notes` (or similar) — add a note documenting the observed delta from the old hand-entered curves to the new bootstrapped curves. Example: "Phase 2: discount and hazard curves now bootstrapped from raw S531 swap-curve and CDX par-spread quotes (was: hand-entered knot points). Bootstrapped DFs differ from the prior knots by O(1e-6); par spreads by O(1e-7). Tolerances widened from <old> to <new> to accommodate."

- [ ] **Step 5: Re-run the golden suite — must pass**

Run: `cargo test -p finstack-valuations --test golden cds_option`
Run: `uv run pytest -v finstack-py/tests/golden/test_pricing_cds_option.py`
Expected: PASS, with the new tolerances accommodating the bootstrap-vs-snapshot delta.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/tests/golden/data/pricing/cds_option/cdx_ig_46_payer_atm_jun26.json
git commit -m "test(valuations): migrate CDX IG 46 fixture to market_envelope

Replace the embedded MarketContext snapshot with a CalibrationEnvelope
that bootstraps the USD-S531 swap discount curve and the CDX.NA.IG.46
hazard curve from raw quotes (S531 deposit/swap rates and CDX par
spreads). The SABR vol surface continues to ship as a materialized
snapshot in initial_market.surfaces (calibrating it from raw vol
quotes is deferred to a later sweep).

Bloomberg expected_outputs are unchanged. actual_outputs_under_bloomberg_
quadrature and tolerances were remeasured against the new bootstrapped
base curves; provenance.notes documents the observed delta."
```

---

## Task 11: Notebook expansion — three new sections

**Goal:** Expand `market_bootstrap_tour.ipynb` from the Phase 1 single-flow scaffold into a fuller analyst walkthrough with composition, snapshot-data, and accessor sections, plus a placeholder pointer to Phase 4's `dry_run`.

**Files:**
- Modify: `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb`

- [ ] **Step 1: Read the existing notebook**

The Phase 1 notebook (committed `d7a385c5a`) has 6 cells loading `01_usd_discount.json`. Phase 2 keeps these cells and adds three new sections after them, plus a placeholder.

- [ ] **Step 2: Author additions via `nbformat`**

Use `uv run python` with `nbformat` to load, append, and save:

```python
import nbformat as nbf
from pathlib import Path

NB_PATH = Path("finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb")
nb = nbf.read(NB_PATH, as_version=4)

# Append new cells in order.
new_cells = [
    nbf.v4.new_markdown_cell("## Compose markets — chained envelope\n\n"
        "An analyst's morning bootstrap typically chains discount → hazard → base correlation\n"
        "in a single envelope. `12_full_credit_desk_market.json` shows the full pattern:\n"
        "rates and CDS quotes feed two calibration steps, and a tranche quote_set drives the\n"
        "third. FX cross rates ride along in `initial_market` since they're snapshot-only data."),
    nbf.v4.new_code_cell(
        "envelope_path = REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / \"12_full_credit_desk_market.json\"\n"
        "envelope_json = envelope_path.read_text()\n"
        "result = calibrate(envelope_json)\n"
        "print(f\"success: {result.success}\")\n"
        "print(f\"steps: {result.step_ids}\")\n"
        "print(f\"rmse: {result.rmse:.3e}\")\n"
        "result.report_to_dataframe()"
    ),
    nbf.v4.new_code_cell(
        "market = result.market\n"
        "discount = market.get_discount(\"USD-OIS\")\n"
        "hazard = market.get_hazard(\"CDX-NA-IG-46\")\n"
        "bc = market.get_base_correlation(\"CDX-NA-IG-46-BASE-CORR\")\n"
        "print(f\"USD-OIS DF(5y):  {discount.df(5.0):.6f}\")\n"
        "print(f\"CDX-IG-46 SP(5y): {hazard.sp(5.0):.6f}\")\n"
        "print(f\"BC at 7%:        {bc.correlation(0.07):.4f}\")"
    ),
    nbf.v4.new_markdown_cell("## Snapshot-only data — FX, bonds, equities\n\n"
        "FX matrices, bond prices, equity spots, and dividend schedules are not bootstrapped\n"
        "today — they ride in `initial_market`. The reference envelopes 09 / 10 / 11\n"
        "demonstrate each pattern."),
    nbf.v4.new_code_cell(
        "for name in [\"09_fx_matrix.json\", \"10_bond_prices.json\", \"11_equity_spots_dividends.json\"]:\n"
        "    path = REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / name\n"
        "    result = calibrate(path.read_text())\n"
        "    print(f\"{name}: success={result.success}, steps={result.step_ids}\")"
    ),
    nbf.v4.new_code_cell(
        "# FX cross rate (triangulated through USD pivot).\n"
        "fx_envelope = (REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / \"09_fx_matrix.json\").read_text()\n"
        "fx_market = calibrate(fx_envelope).market\n"
        "# `fx_market.fx_rate(...)` may need an as-of date depending on the binding shape;\n"
        "# adjust to whichever signature the Python MarketContext exposes.\n"
        "# Bond prices.\n"
        "bond_envelope = (REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / \"10_bond_prices.json\").read_text()\n"
        "bond_market = calibrate(bond_envelope).market\n"
        "for bond_id in [\"US-TREASURY-10Y-2026-05-08\", \"IBM-7YR-2033\"]:\n"
        "    print(f\"{bond_id}: {bond_market.get_price(bond_id)}\")"
    ),
    nbf.v4.new_markdown_cell("## Validate before solving\n\n"
        "Phase 4 will add `dry_run(envelope_json)` for fast pre-flight validation\n"
        "(missing dependencies, undefined quote sets, quote class mismatches) — \n"
        "running structural checks in microseconds before the (much slower) solver.\n"
        "For now, `validate_calibration_json` returns the canonical pretty-printed JSON\n"
        "and surfaces serde-level errors; it is the only validation tool until Phase 4 lands."),
    nbf.v4.new_code_cell(
        "from finstack.valuations import validate_calibration_json\n"
        "envelope_path = REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / \"01_usd_discount.json\"\n"
        "canonical = validate_calibration_json(envelope_path.read_text())\n"
        "print(canonical[:200] + \"...\")"
    ),
]

nb.cells.extend(new_cells)

# Strip any execution outputs from the appended cells.
for c in nb.cells:
    if c.cell_type == "code":
        c.outputs = []
        c.execution_count = None

nbf.write(nb, NB_PATH)
print(f"Notebook now has {len(nb.cells)} cells.")
```

This adds 8 cells (alternating markdown / code) to the existing 6, for a total of 14.

- [ ] **Step 3: Verify the notebook executes end-to-end**

Run: `uv run jupyter nbconvert --to notebook --execute finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb --output executed_market_bootstrap_tour.ipynb`

If `jupyter nbconvert` is not on PATH, fall back to: `uv run --with jupyter --with ipykernel jupyter nbconvert ...`.

Expected: all 14 cells execute without error. Delete the executed-output copy after verification.

If a cell errors due to an accessor name mismatch (e.g., `market.fx_rate` not the right method), update the affected code cell to match the real Python `MarketContext` API and re-run.

- [ ] **Step 4: Sanity check structure**

Run:

```bash
python -c "
import nbformat
nb = nbformat.read('finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb', as_version=4)
print(f'cells: {len(nb.cells)}')
print(f'all code cells unexecuted: {all(c.execution_count is None for c in nb.cells if c.cell_type == \"code\")}')
"
```

Expected: `cells: 14` and `all code cells unexecuted: True`.

- [ ] **Step 5: Commit**

```bash
git add finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb
git commit -m "docs(finstack-py): expand market_bootstrap_tour with composition + snapshot-data sections

Phase 2 notebook expansion: adds three new sections beyond the Phase 1
single-flow scaffold:

- Compose markets — uses 12_full_credit_desk_market.json to demonstrate
  the canonical analyst-morning chained-plan pattern (discount →
  hazard → base correlation).
- Snapshot-only data — uses 09 / 10 / 11 to demonstrate FX cross rates,
  bond price lookup, and equity spots.
- Validate before solving — current state (validate_calibration_json),
  with a forward pointer to Phase 4's dry_run() for richer pre-flight
  checks.

Notebook total: 14 cells. All cells execute via jupyter nbconvert."
```

---

## Task 12: End-to-end verification & spec acceptance check

**Goal:** Confirm Phase 2 is complete with no regressions; walk through the spec's 5 acceptance criteria.

- [ ] **Step 1: Run focused tests**

```bash
cargo test -p finstack-valuations --test calibration reference_envelopes
cargo test -p finstack-valuations --test golden cds_option
uv run pytest -v finstack-py/tests/golden/test_pricing_cds_option.py
```

All must pass. The reference-envelope test count should be 12 (was 3 in Phase 1).

- [ ] **Step 2: Run the full project verification stack**

```bash
mise run all-fmt
mise run all-lint
mise run python-lint
mise run wasm-lint
mise run all-test
```

Expected: green across the board.

- [ ] **Step 3: Walk the spec's acceptance criteria**

From `docs/2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md` §6:

- [ ] All twelve envelope files exist under `finstack/valuations/examples/market_bootstrap/` and parse as `CalibrationEnvelope`.
- [ ] Twelve integration tests pass, each demonstrating a representative accessor query on the produced `MarketContext`.
- [ ] CDX IG 46 fixture uses `market_envelope`. Pricing golden passes. Bloomberg `expected_outputs` unchanged; finstack reconciliation notes updated.
- [ ] Notebook has at least three additional sections covering composition, snapshot data, and accessor patterns.
- [ ] No regressions: full test suite passes.

If any criterion fails, return to the relevant task before declaring Phase 2 complete.

- [ ] **Step 4: Final commit if any cleanup needed**

If the verification stack flagged any formatter/lint adjustments, stage and commit as a final cleanup commit:

```bash
git status
git add <fixed-files>
git commit -m "chore: format and lint cleanup for market bootstrap phase 2"
```

---

## Phase 2 done

When this plan is complete, the reference catalog is full: twelve canonical envelopes covering every commonly-needed curve, surface, and snapshot type, the production CDX IG 46 fixture exercises the bootstrap path end-to-end, and the analyst notebook walks the full canonical workflow.

Phase 3 (IDE autocomplete via JSON Schema + TypeScript types) is the natural next slice. See [`docs/2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md`](2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md).
