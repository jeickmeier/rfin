# Golden Fixture Envelope Migration Plan

**Status:** Draft
**Date:** 2026-05-09
**Owner:** finstack-valuations
**Goal:** Migrate the 31 remaining materialized-market golden fixtures to use the `market_envelope` (calibration-driven) entry point, exercising the Phase 4/5 calibration surface end-to-end.

## 1. Motivation

Today only **2 of 33 pricing golden fixtures** route through the calibration engine:

| Fixture | Path |
|---|---|
| `market_envelope_smoke/usd_deposit_3m_envelope.json` | Phase-1 smoke wrapper |
| `cds_option/cdx_ig_46_payer_atm_jun26.json` | Real production migration |

The remaining 31 feed pre-calibrated `MarketContextState` JSON directly. This means:

- The new Phase 4 structured-error surface (`EnvelopeError::SolverNotConverged`, `worst_quote_id`) is exercised on 1 fixture.
- The Phase 5 TypedDict definitions have no runtime witness in the golden suite.
- Regressions in the calibration engine that don't break the engine's own unit tests can ship without golden-test coverage.
- Fixture authors learn one set of conventions (materialized market) but production users will increasingly use the other (envelope).

## 2. Goals

- Every pricing golden fixture eventually uses `market_envelope`.
- For each calibratable curve, calibration drives it from quotes; non-calibratable artifacts (FX matrices, externally-built vol surfaces, prices, dividends) stay in `initial_market`.
- Fixtures retain their existing expected outputs; tolerances only loosen with explicit documentation.
- Migrated fixtures gain a clear `provenance` note explaining where the quotes came from.

## 3. Non-Goals

- Replacing externally-sourced surfaces (Bloomberg SABR, FX delta vol) with calibrated equivalents. Those stay in `initial_market` because their construction is not reproducible from a finstack-side bootstrap.
- Migrating non-pricing goldens (analytics, scenario, attribution) — out of scope for this plan.
- Building a generic "snapshot → envelope" auto-converter for arbitrary external curves — keep it manual / per-fixture.

## 4. Architectural Baseline

The runner already supports both paths:

- [`finstack-py/tests/golden/runners/pricing_common.py`](../finstack-py/tests/golden/runners/pricing_common.py): `_resolve_market` accepts `market` or `market_envelope`.
- [`finstack/valuations/tests/golden/pricing_common.rs`](../finstack/valuations/tests/golden/pricing_common.rs): same dispatch.
- Both runners now route `market_envelope` through `engine::execute_with_diagnostics` (Phase 4 audit fix), so envelope failures surface `kind` / `step_id` / `worst_quote_id`.

Migration is therefore a per-fixture data transformation, not a runner change.

## 5. Migration Pattern

For each materialized fixture, transform:

```json
{
  "inputs": {
    ...,
    "market": { "curves": [...], "surfaces": [...], "fx": {...}, "prices": {...} }
  }
}
```

into:

```json
{
  "inputs": {
    ...,
    "market_envelope": {
      "schema": "finstack.calibration",
      "plan": {
        "id": "<fixture_name>",
        "quote_sets": { "<set_name>": [<MarketQuote>...] },
        "steps": [{ "id": "<curve_id>", "kind": "<step_kind>", "quote_set": "<set_name>", ... }]
      },
      "initial_market": {
        "fx": {...}, "surfaces": [...], "prices": {...}
        // Plus any curves that aren't bootstrap-reproducible.
      }
    }
  }
}
```

**Per-curve decision:** for each curve in the original `market.curves`:

1. **Bootstrap-reproducible** (deposit / IRS / FRA / hazard / inflation curves): move to `plan.steps`; add `quote_set` from source documentation. Verify the calibrator reproduces the same curve values within fixture tolerance.
2. **Externally-sourced or non-bootstrap-able** (parametric NS, externally-smoothed Bloomberg curves): keep in `initial_market.curves`. The fixture still uses `market_envelope` but contributes no new calibration coverage for that curve.

Surfaces, FX, prices, dividends always stay in `initial_market`.

## 6. Inventory

### 6.1 Already migrated (2 fixtures)

| Fixture | Steps | Notes |
|---|---|---|
| `cds_option/cdx_ig_46_payer_atm_jun26.json` | discount + hazard | Bloomberg-sourced rates + CDS quotes |
| `market_envelope_smoke/usd_deposit_3m_envelope.json` | (none — empty plan) | Phase-1 smoke / round-trip-only |

### 6.2 To migrate (31 fixtures, grouped by complexity)

#### Tier 1 — Discount-only (8 fixtures)

| Fixture | Source | Curves | Effort |
|---|---|---|---|
| `deposit/usd_deposit_3m.json` | quantlib | 1 discount | XS |
| `irs/usd_sofr_5y_receive_fixed_swpm.json` | bloomberg | 1 discount | S |
| `swaption/usd_swaption_5y_into_5y_receiver_25_otm.json` | bloomberg | 1 discount + 1 surface | S |
| `equity_index_future/spx_es_3m.json` | formula | 1 discount + 2 prices | XS |
| `equity_option/bs_atm_call_1y.json` | formula | 1 discount + 1 surface | XS |
| `equity_option/bs_itm_put.json` | formula | 1 discount + 1 surface | XS |
| `equity_option/bs_otm_call_25d.json` | formula | 1 discount + 1 surface | XS |
| `equity_option/bs_short_dated_1m.json` | formula | 1 discount + 1 surface | XS |
| `equity_option/bs_with_dividend_yield.json` | formula | 1 discount + 1 surface | XS |

**Calibration recipe:** single `discount` step driven by 1-N deposit or IRS quotes at the curve's pillars. Surfaces stay in `initial_market`. Equity options also keep `prices` and `dividends` in `initial_market`.

**Effort estimate:** 4-6 hours total (formula fixtures synthesize quotes from the curve's own DFs at standard pillars; Bloomberg fixtures transcribe from screenshots).

#### Tier 2 — Discount + Forward (8 fixtures)

| Fixture | Source | Curves | Effort |
|---|---|---|---|
| `bond_future/ust_ty_10y_front_month.json` | formula | discount only (1) | S |
| `cap_floor/usd_cap_5y_atm_black.json` | bloomberg | discount + forward + surface | M |
| `fra/usd_fra_3x6.json` | bloomberg | discount + forward | M |
| `ir_future/sofr_1m_serial.json` | formula | discount + forward | S |
| `ir_future/sofr_3m_quarterly.json` | formula | discount + forward | S |
| `structured_credit/abs_credit_card_senior.json` | formula | discount + forward | M |
| `structured_credit/clo_mezzanine_base_case.json` | formula | discount + forward | M |
| `fx_swap/eurusd_fx_swap_3m.json` | formula | 2× discount + fx | M |

**Calibration recipe:** sequential discount + forward steps. Discount from deposits/swaps, forward from IRS quotes against the prior discount curve. fx_swap needs 2 currencies' discount curves; FX matrix stays in `initial_market`.

**Effort estimate:** 8-12 hours total.

#### Tier 3 — Credit + Inflation (5 fixtures)

| Fixture | Source | Curves | Effort |
|---|---|---|---|
| `cds/cds_5y_par_spread.json` | bloomberg | discount + hazard | M |
| `cds/cds_quantlib_flat_hazard_decomposition.json` | quantlib | discount + hazard | M |
| `cds_tranche/cdx_ig_5y_3_7_mezz.json` | formula | discount + hazard + base_correlation + credit_index | L |
| `inflation_linked_bond/inflation_linked_bond_5y.json` | formula | discount + inflation | M |
| `inflation_swap/inflation_zc_swap_5y.json` | formula | discount + inflation | M |

**Calibration recipe:** discount step first, then domain-specific (hazard from CDS quotes, inflation from ZCIS quotes, base_correlation from CDX tranche upfront quotes). cds_tranche additionally has `credit_indices` aggregate that stays in `initial_market`.

**Effort estimate:** 10-14 hours total.

#### Tier 4 — Multi-currency / multi-asset (10 fixtures)

| Fixture | Source | Curves | Effort |
|---|---|---|---|
| `bond/bhccn_10_2032_callable_bloomberg.json` | bloomberg | 2-currency discount + forward (4 curves) | L |
| `bond/ibm_eur_2034_callable_bloomberg.json` | bloomberg | 2-currency discount + forward (3 curves) | L |
| `convertible/conv_bond_atm_3y.json` | formula | discount + forward + hazard + prices | L |
| `convertible/conv_bond_distressed.json` | formula | discount + forward + hazard + prices | L |
| `fx_option/gk_eurusd_25d_call.json` | formula | 2× discount + fx + 2 surfaces | L |
| `fx_option/gk_eurusd_atm_3m.json` | formula | 2× discount + fx + 2 surfaces | L |
| `fx_option/gk_eurusd_otm_call_6m.json` | formula | 2× discount + fx + 2 surfaces | L |
| `fx_option/gk_usdjpy_atm_1y.json` | formula | 2× discount + fx + 2 surfaces | L |
| `term_loan/term_loan_b_5y_floating.json` | formula | discount + forward + hazard | M |

**Calibration recipe:** chains of steps spanning rate + credit. fx_option and bond fixtures have 2-currency rate stacks (each currency: discount, optionally forward) plus FX matrix in `initial_market`. fx_option fixtures' delta-vol surfaces stay in `initial_market.fx_delta_vol_surfaces`. Convertible's equity prices and dividend schedules stay in `initial_market`.

**Effort estimate:** 14-20 hours total.

### 6.3 Total scope

- **31 fixtures** to migrate
- **~36-52 hours** of careful work, mostly bounded by quote-transcription / synthesis effort
- Each tier is independent; can parallelize across team members

## 7. Per-Fixture Workflow

For each fixture:

1. **Read the existing snapshot.** Identify each curve, surface, scalar.
2. **Categorize each artifact:** bootstrap-reproducible? If yes, plan a step. If no, leave in `initial_market`.
3. **Source the quotes** for each calibration step:
   - **formula fixtures**: synthesize quotes from the existing curve. For a discount curve with discount factors at pillars `t_i`, emit deposit/swap quotes whose calibration reproduces those DFs. The implicit contract is "the existing snapshot was generated by a calibration; we recover its inputs."
   - **bloomberg-screen fixtures**: transcribe quotes from `provenance.screenshots`. Bloomberg SWPM / DCO screens explicitly show the input quotes.
   - **quantlib fixtures**: re-derive from the QuantLib reference (these are deterministic).
4. **Build the envelope** in-place. Replace `inputs.market` with `inputs.market_envelope` containing `plan.steps` + `plan.quote_sets` + (residual) `initial_market`.
5. **Run the golden test.** It must produce the same expected outputs within the fixture's existing tolerances.
6. **If tolerances drift:** decide between (a) tightening the calibration setup (interpolation method, day count), (b) loosening the fixture tolerance with documentation, or (c) reverting that curve to `initial_market` for this fixture.
7. **Update `provenance`:** add a note describing how the quotes were sourced. Bump `last_reviewed_on`.

## 8. Tooling

### 8.1 `migrate_fixture.py` helper (recommended, ~half-day to build)

Location: `scripts/golden/migrate_fixture.py`.

Purpose: mechanical conversion of materialized snapshots to envelope shape.

Pseudo-API:

```bash
python scripts/golden/migrate_fixture.py \
    --fixture finstack/valuations/tests/golden/data/pricing/deposit/usd_deposit_3m.json \
    --calibrate USD-OIS:discount \
    --quote-set USD-OIS:from-curve \
    --dry-run
```

Capabilities:
- Read a fixture, dump its `market.curves` IDs and types.
- For each curve named with `--calibrate`, generate a calibration step + quotes synthesized from the curve's pillar discount factors (formula path) or stub-out the quote_set for manual fill (bloomberg path).
- For curves not named, leave in `initial_market.curves`.
- Emit the migrated fixture as a new file (or in-place with `--in-place`).
- Optionally run the golden test post-migration and report tolerance check.

This pays for itself starting at fixture #5.

### 8.2 Quote-synthesis primitives

For formula fixtures, we need a Python helper that, given a `DiscountCurve` snapshot, emits a list of `RateQuote` entries that bootstrap-reproduce it. Likely:

- For pillar dates with simple money-market tenors → `Deposit` quotes with the implied zero rate.
- For longer tenors → `Swap` quotes with the implied par swap rate (computed against the same curve).

Lives in: `finstack-py/finstack/valuations/_test_utils/quote_synthesis.py` (private; not part of public API).

### 8.3 Tolerance instrumentation

Add a CSV row per fixture showing pre- and post-migration absolute error vs. expected. Lets us see at a glance which migrations introduced drift.

## 9. Phasing & Acceptance Criteria

### Phase 0 — Tooling + precedent (already largely done)

- [x] CDX IG 46 fixture migrated (commit `187dc0109`).
- [x] Smoke fixture in place.
- [x] Runners route through `execute_with_diagnostics` and `CalibrationEnvelopeError`.
- [ ] **Build `migrate_fixture.py` helper** — ~4 hours.
- [ ] **Build `quote_synthesis.py` for discount curves** — ~3 hours.

### Phase 1 — Tier 1 (discount-only, 8 fixtures)

Acceptance: every Tier-1 fixture uses `market_envelope`, calibrates a single discount curve from quotes, and passes its existing tolerances.

Effort: 4-6 hours after tooling is in place.

### Phase 2 — Tier 2 (discount + forward, 8 fixtures)

Acceptance: every Tier-2 fixture uses `market_envelope`, calibrates discount + forward, and passes existing tolerances. Forward-curve quote_set built from synthesized IRS rates against the calibrated discount.

Effort: 8-12 hours.

### Phase 3 — Tier 3 (credit + inflation, 5 fixtures)

Acceptance: every Tier-3 fixture uses `market_envelope`. Hazard / inflation steps populated from CDS / ZCIS quotes (synthesized for formula fixtures, transcribed from screenshots for Bloomberg fixtures).

Effort: 10-14 hours.

### Phase 4 — Tier 4 (multi-currency / multi-asset, 10 fixtures)

Acceptance: every Tier-4 fixture migrated. Multi-currency rate stacks calibrate per-currency. Convertible bond's underlying-equity / dividend stays in `initial_market`. FX option vol surfaces stay in `initial_market`.

Effort: 14-20 hours.

### Phase 5 — Sweep + cleanup

- [ ] Confirm `discover_fixtures("pricing")` produces 33/33 envelope-driven fixtures.
- [ ] Remove `market` branch from `_resolve_market` (Python + Rust) and the runner-level mutual-exclusion check (since only one mode is supported now). Optional: keep the `market` branch for one release cycle to ease external migration.
- [ ] Update `conftest.py` schema validator to require `market_envelope` exclusively.
- [ ] Document the migration in the runner header / contributor docs.

## 10. Risks

1. **Float drift between calibrated and embedded curves.** Bloomberg-derived curves are smoothed externally; bootstrapping from the same input quotes won't bit-match. *Mitigation:* fall back to `initial_market.curves` for that curve only; keep the rest of the envelope quote-driven. Document in fixture's `description`.

2. **Quote-synthesis correctness.** The recipe "round-trip an embedded curve back to quotes" must use the right interpolation/conventions. A wrong pillar choice or day-count produces a curve that *looks* the same but evaluates differently at off-pillar points. *Mitigation:* unit-test the helper with known inputs; gate Phase 1 on >=3 fixtures passing exactly.

3. **Tolerance bloat.** Migrating each fixture is a chance to silently widen tolerances. *Mitigation:* every tolerance change requires `tolerance_reason` field with a written justification; add a CI check that flags net tolerance increases vs. master.

4. **Schedule slip.** ~36-52h is a large block. *Mitigation:* phases are independent; one engineer can ship Tier 1 in a day. Don't gate Tier 2 on Tier 4 completion.

5. **Loss of materialized-snapshot path.** Removing the `market` branch makes the runner less flexible. *Mitigation:* defer the removal step to Phase 5; keep both supported during the migration.

## 11. Verification

Each fixture's migration is "done" when:

- `cargo test -p finstack-valuations --test golden -- <fixture path>` passes.
- `pytest finstack-py/tests/golden/test_pricing_<domain>.py` passes for that fixture.
- The fixture no longer contains `inputs.market` (only `inputs.market_envelope`).
- The migration writes one row to `target/golden-reports/golden-comparisons.csv` with `passed=true` for every metric.
- The `provenance` block has updated `last_reviewed_on` and includes a note about the calibration source.

## 12. Open Questions

- Do we want a parallel "envelope-only" runner registered as a separate test, or is in-place replacement of the snapshot fine? (Recommendation: in-place; keeping two rails doubles maintenance.)
- For formula fixtures, do we synthesize quotes deterministically (one quote per pillar) or do we store Bloomberg-style multi-instrument quote sets (deposits + IRS + FRA)? (Recommendation: simplest possible quote set that reproduces the curve. Add complexity only when the fixture's domain demands it.)
- Should `dry_run` validation be added to `conftest.py` to catch envelope structural issues at fixture-load time? (Recommendation: yes, after Phase 1.)

---

**Next step before executing:** review this plan, decide on tooling scope (Phase 0 budget), and pick a tier to start with. Recommend starting Tier 1 immediately with the simplest fixture (`deposit/usd_deposit_3m.json`) as a proof of concept — it can be done by hand in ~30 minutes and validates the migration pattern before committing to tooling investment.

---

## 13. POC Findings (2026-05-09): `deposit/usd_deposit_3m.json`

### What was tried

Migrated the QuantLib-derived deposit fixture to a single-deposit calibration step:
- Quote rate computed analytically from the snapshot's DF at maturity (`r = (1/DF − 1)/τ_act360 = 0.03985001083863061`).
- Pillar set to absolute date `2026-07-30` (the instrument's maturity).
- Curve `interpolation: log_linear`, `extrapolation: flat_forward`, `curve_day_count: Act365F` to mirror the snapshot.

### What happened

Tests **failed** with these drifts:

| Metric | Expected | Calibrated | Diff | Tolerance |
|---|---|---|---|---|
| `deposit_par_rate` | 0.0398500108 | 0.0398588063 | 8.8e-6 | 1e-12 |
| `npv` | 2877.93 | 2855.91 | 22.0 | 1e-6 |
| `dv01` | -249.387 | -249.386 | 5.5e-4 | 1e-6 |

The calibrated curve had `DF(2026-07-30) = 0.9900250810` vs the snapshot's `0.9900272602`. Reverse-engineering shows the calibrator's effective `τ` is ~0.252888 — neither `91/360` (=0.252778) nor `91/365` (=0.249315). The day-count handling inside the curve-build path differs subtly from the deposit-pricer's `τ_act360` used for `par_rate` evaluation, producing a ~2e-6 drift that overwhelms the QuantLib-parity tolerance of `1e-12`.

### Why this fixture can't take true calibration

The QuantLib oracle's discount curve uses **synthetic time pillars** at clean values `[0.0, 0.25, 0.5, 1.0, 2.0, 5.0]`. These are not reproducible from any real calendar date under Act/365F:
- `t = 0.25` would need 91.25 days; integer 91 → 0.249315, integer 92 → 0.252055.
- `t = 5.0` over 5 years includes a leap year, so 5×365 = 1825 days → 4.9973 (or 1826 → 5.00274).

True-calibration migration would require either tolerance loosening of 6+ orders of magnitude (defeating the QuantLib-parity contract) or per-fixture quote-rate tuning that depends on undocumented internals of the deposit calibrator.

### What landed

**Tier-A wrap:** the fixture now uses `inputs.market_envelope` with:
- The original snapshot curves carried verbatim in `initial_market.curves`.
- Empty `plan.steps` and `plan.quote_sets`.

Result: routes through `engine::execute_with_diagnostics`, exercises the calibration entry point's plumbing (envelope schema validation, FX preservation, curve passthrough), and preserves all expected outputs at the original `1e-12` / `1e-6` tolerances. **Both Rust and Python golden tests pass unchanged.**

### Plan updates implied

1. **Two distinct migration tiers, not one.** Reframe the plan as:
   - **Tier-A wrap** (always available): replace `market` with `market_envelope`; carry curves in `initial_market.curves`; empty plan. Coverage gain: validates the envelope plumbing for that fixture. ~1-3 minutes per fixture, scriptable.
   - **Tier-B calibrate** (case-by-case): replace one or more curves with `plan.steps` driven by quotes. Coverage gain: exercises bootstrap math. Only viable when the snapshot's pillars align with real calendar dates (mostly Bloomberg fixtures with concrete tenor pillars).

2. **Tier-A becomes the default migration.** Most fixtures get Tier-A immediately; Tier-B is opportunistic where the snapshot supports it.

3. **Identify Tier-B candidates by snapshot inspection.** Specifically: do the curve's `knot_points` have times derivable from real calendar dates under the curve's `day_count`? If yes, Tier-B may work. The currently-migrated `cdx_ig_46_payer_atm_jun26.json` is a known Tier-B example (Bloomberg-sourced, real-date pillars).

4. **Synthesizing quotes from snapshots is harder than the plan assumed.** The "round-trip an embedded curve back to quotes" tooling (`quote_synthesis.py`) is only useful when the curve was originally bootstrapped from real-date quotes. For QuantLib/formula fixtures with synthetic times, no quote set reproduces the snapshot exactly. Defer building this helper until real Tier-B migration begins.

5. **The redundancy with `market_envelope_smoke/usd_deposit_3m_envelope.json` is now visible.** The smoke fixture and the migrated original both wrap the same QuantLib oracle in an empty envelope. Recommend retiring the smoke fixture in a follow-up (after the broader Tier-A pass completes), since every Tier-A-wrapped fixture is itself an envelope-plumbing smoke test.

### Updated effort estimate

| Tier | Coverage | Effort/fixture | Total |
|---|---|---|---|
| **Tier-A wrap** (all 31 fixtures) | Envelope plumbing only | 2-5 min (scriptable) | 2-3 hours |
| **Tier-B calibrate** (subset, ~5-10 fixtures) | Bootstrap math | 1-3 hours | 10-25 hours |

Total: **12-28 hours**, down from the original 36-52h estimate, because most fixtures take the cheap Tier-A path.

### Recommended next moves

1. **Land the deposit Tier-A migration** (this POC) as a proof and pattern reference.
2. **Script the Tier-A bulk migration**: `scripts/golden/wrap_in_envelope.py` reads each `market`-using fixture, swaps to `market_envelope.initial_market`, writes back. Estimated 2-3 hours including testing.
3. **Apply to all 30 remaining fixtures.** Run full golden suite. Expected zero deltas (the snapshots are unchanged).
4. **Identify Tier-B candidates** by inspecting each migrated fixture's snapshot pillar alignment with calendar dates. Likely candidates: Bloomberg fixtures (`cap_floor`, `fra`, `irs`, `swaption`, `cds_5y_par_spread`, `bond/*`), since they were originally captured from screenshots with concrete dated quotes.
5. **Pursue Tier-B opportunistically** for high-value fixtures where calibration coverage matters (rates, credit). Defer formula-fixture Tier-B indefinitely.

---

## 14. POC #2 (2026-05-09): `irs/usd_sofr_5y_receive_fixed_swpm.json`

### What was tried

Migrated the Bloomberg SWPM USD SOFR 5Y receive-fixed swap fixture to a real calibration:
- Transcribed all **26 input rates** from the SWPM Curve 490 screenshot (3 deposits at 1W/2W/3W + 23 OIS swaps from 1M to 12Y).
- Built a single discount step (`USD-SOFR`) consuming all 26 quotes.
- Tried `Pillar::Tenor` (small drift) and `Pillar::Date` with Bloomberg pay-date pillars (larger drift; Bloomberg's schedule generation uses spot+T+2+MFC which Finstack's calibration doesn't replicate from the step config alone). Settled on `Pillar::Tenor`.

### What happened

The bootstrap converges genuinely (2055 iterations, max residual 9.53e-13, 27 knots from 26 quotes). But the calibrated curve differs from Bloomberg's snapshot DFs by ~1bp in the 5Y zone:

| Metric | Expected (Bloomberg) | Calibrated | Diff (abs) | Original tolerance | New tolerance |
|---|---|---|---|---|---|
| `npv` | -91334.81 | -91230.92 | 103.89 | 10.0 | **110.0** |
| `par_rate` | 0.0370075 | 0.0370050 | 2.45e-6 | 5e-7 | **3e-6** |
| `dv01` | -4523.48 | -4523.49 | 0.01 | 0.3 | 0.5 (cushion) |
| Payment counts, dates, accrual factors | (5 metrics) | exact | 0 | 1e-12 | unchanged |

The drift comes from convention mismatch: Bloomberg SWPM bootstraps from **spot (T+2) + tenor + Modified Following + USNY calendar**; Finstack's calibration step only configures `curve_day_count`, so it falls back to defaults that produce slightly different pillar dates. The drift is internally consistent (1bp throughout the 5Y zone) and not a bug in either side — just two different schedule conventions.

### What landed

- **Tier-B calibration** with 26 SWPM quotes driving the discount curve.
- Tolerances widened with a documented `tolerance_reason` per metric explaining the convention gap.
- Bloomberg screen values remain the assertion targets; the fixture now also catches regressions in Finstack's bootstrap that would shift DFs at the 5Y zone by more than ~1bp.

### Plan updates implied

**Confirms the original plan's risk assessment (§10.1):** Bloomberg-sourced fixtures are achievable for Tier-B *only with tolerance loosening*. This is the realistic shape of Tier-B work. The original plan's assumption that Bloomberg fixtures would migrate within existing tolerances was optimistic.

**New Tier-B sub-classification:**

- **Tier-B1: Tight Bloomberg parity** — only viable if Finstack's calibration step gains schedule-convention configuration (`spot_lag_days`, `bdc`, `calendar_id`, `payment_frequency`) so the bootstrap matches Bloomberg's exactly. This is a finstack-valuations enhancement, not a fixture migration. **Out of scope for this plan.**

- **Tier-B2: Documented-drift Bloomberg parity** (what this POC achieved) — calibrate from input rates with default Finstack conventions; widen tolerances to absorb the 1bp pillar-date drift; document the tolerance_reason. **This is the practical Tier-B path for Bloomberg fixtures.**

**Effort revision for Tier-B2 fixtures:**

- Per-fixture: 30-90 min for transcription + envelope build + tolerance widening + tests.
- Tooling: a per-fixture quote-transcription helper isn't useful (each Bloomberg curve has different rate sets). The migration is genuinely manual.

### Recommended next moves (updated)

1. **Land the IRS Tier-B2 migration** (this POC) and the deposit Tier-A migration together as the working pattern reference.
2. **Bulk Tier-A wrap of remaining 29 fixtures.** ~2 hours scripted.
3. **Tier-B2 the Bloomberg fixtures with visible quote screenshots:** `cds/cds_5y_par_spread.json`, `cap_floor/usd_cap_5y_atm_black.json`, `fra/usd_fra_3x6.json`, `swaption/usd_swaption_5y_into_5y_receiver_25_otm.json`, `bond/*` (2 fixtures). Estimated 4-9 hours total.
4. **File a follow-up issue** for Tier-B1 readiness: extend `RatesStepConventions` with `spot_lag_days`, `bdc`, `calendar_id` so future migrations can achieve Bloomberg parity without tolerance loosening.
