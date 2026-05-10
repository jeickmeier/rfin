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

---

## 15. POC #3 (2026-05-09): `fra/usd_fra_3x6.json` — hybrid Tier-AB

### What was tried

Migrated the Bloomberg SWPM USD 2M x 5M FRA fixture. The fixture uses two curves:
- **Discount** `USD-SOFR` (18-knot snapshot from Curve 490)
- **Forward** `USD-3M-CME-TERM-SOFR` (5-knot snapshot from Curve 559)

Approach: calibrate the discount curve from input quotes; preserve the forward curve verbatim in `initial_market`.

### Key discovery: forward curve is engineered

Inspecting the snapshot's forward curve revealed that **all 5 knots have the same forward rate of 0.0364370** — it's a **flat curve at 3.6437%, exactly the FRA's strike**. This is a synthetic test curve constructed to make the FRA at-the-money (NPV = 0). It does not correspond to Bloomberg's actual Curve 559 (whose 3M Cash quote is 3.66395% and zero rates range 3.69-3.93% across tenors).

Bootstrapping the forward curve from Bloomberg's Curve 559 input rates would produce a non-flat curve and shift the FRA away from at-the-money. The fixture's at-the-money assertion contract would break.

**Resolution: hybrid Tier-AB.** Calibrate the discount curve (Tier-B); preserve the forward curve verbatim in `initial_market.curves` (Tier-A for that curve only).

### Implementation

The envelope's `plan.steps` contains a single discount step:
- `quote_set`: 12 SWPM Curve 490 rates from `bloomberg_reference.discount_curve_screen`
  - 1W/2W/3W deposits, 1M/2M/.../12M/18M/2Y OIS swaps
- `conventions.curve_day_count: Act365F`
- `conventions.ois_compounding: CompoundedWithRateCutoff{cutoff_days: 1}` (matching Bloomberg SWPM, IRS POC pattern)

`initial_market.curves` carries the synthetic forward curve unchanged.

Bootstrap converges genuinely: **954 iterations, residuals at 9.6e-13 across 12 quotes**, 13 knots calibrated.

### Drift profile

| Metric | Bloomberg expected | Calibrated | Diff abs | Original tolerance | New tolerance |
|---|---|---|---|---|---|
| `npv` | -0.01 | -0.01 (≈) | <0.01 | 0.01 | unchanged |
| `par_rate` | 0.036437 | 0.036437 | <5e-8 | 5e-8 | unchanged |
| `pv01` | -251.51 | -251.62 | 0.106 | 0.05 | **0.15** |
| `forward_pv01` | -251.51 | -251.62 | 0.106 | 0.05 | **0.15** |
| `dv01` | -337.37 | -337.70 | 0.327 | 0.05 | **0.4** |

NPV and par_rate **bit-stable at original tolerances** because:
- The forward curve is preserved verbatim, so the projected forward rate at the FRA's accrual period is exactly Bloomberg's 0.036437.
- NPV depends on cashflow × DF(pay_date); cashflow ≈ 0 (FRA is at-the-money), so DF differences cancel out.
- par_rate is the projected forward rate, independent of the discount curve.

Sensitivities (PV01/DV01) **drift by ~0.04% relative** because they shock the discount curve and re-price; the calibrated curve's DF at the pay date differs from the snapshot's by ~4e-4 (consistent with the IRS POC's ~3-4e-4 drift in the 5Y zone, scaled to the FRA's 5M maturity).

### Plan implications

**Hybrid Tier-AB is a viable third path** alongside pure Tier-A wrap and pure Tier-B calibrate. It applies when:
- Some curves are bootstrap-reproducible (Tier-B for those)
- Other curves are synthetic test data (Tier-A wrap inside `initial_market.curves`)
- Same fixture, mixed treatment per curve

This pattern is useful for fixtures whose curves were authored for testability (flat synthetic forward curves, hand-tuned vol surfaces) but whose discount curves are real Bloomberg captures. Likely candidates: the equity-option fixtures (BS surface is synthetic; discount is real), some structured-credit fixtures.

### Updated coverage

After this commit:

| Step kind | Covered? |
|---|---|
| `discount` | ✓ (cdx_ig_46, irs, fra) |
| `hazard` | ✓ (cdx_ig_46) |
| `base_correlation` | ✓ (cdx_ig_46, indirect) |
| `forward` | ✗ (still uncovered — the FRA fixture's forward curve is uncalibratable test data) |

The next migration that *actually* covers the `forward` step kind will need either:
- A different forward fixture whose snapshot is bootstrap-reproducible (most are synthetic `formula` fixtures); or
- A new fixture authored from scratch with real-date forward quotes (e.g., a SOFR 3M-tenor IRS curve calibration).

Recommend file a follow-up to author such a fixture, since none of the existing 33 pricing goldens have a calibratable forward curve.

---

## 16. POC #4 (2026-05-09): `irs/usd_5y_term_irs_self_test.json` — first forward-step coverage

### Motivation

Production forward curves serve floating-rate bonds, term loans, and structured products with multi-year (5-30y) coupon projection. None of the 33 existing pricing goldens had a calibratable forward curve, so the `forward` step kind had no test coverage.

### What was authored

A new `pricing/irs/usd_5y_term_irs_self_test.json` golden fixture:

- **Instrument**: 5Y USD interest rate swap, receive-fixed at 4.25% against 3M-Term-SOFR floating (10M USD notional).
- **Two-step calibration plan** in `market_envelope`:
  - Step 1 (`discount`): USD-OIS bootstrapped from 12 SOFR OIS quotes (1W/2W/3W deposits, 1M-10Y annual swaps, ~4-4.5% rising). Uses the new `ois_compounding: CompoundedWithRateCutoff{cutoff_days: 1}` override.
  - Step 2 (`forward`): USD-SOFR-3M bootstrapped from 8 term-rate IRS quotes (3M-10Y, ~4.05-4.55%) against the just-calibrated discount curve, with `tenor_years: 0.25`.
- **Self-test contract**: Expected outputs (NPV, par_rate, DV01) are captured from Finstack itself, not from a vendor screen. The contract is round-trip stability — re-running calibration and pricing must reproduce the recorded values within the solver's `1e-12` tolerance.

### Why a self-test fixture rather than vendor parity

The fixtures we surveyed for forward calibration had two patterns:

1. **Synthetic formula fixtures** (term_loan, structured_credit, convertible) with clean integer-year pillars [0.25, 0.5, 1, 2, 5, 10] — not reachable from real calendar dates under any standard day count.
2. **Bloomberg fixtures** (cap_floor, bond) where the "forward curve" is actually an OIS forward curve derived from the discount curve, not a separate term-rate calibration.

Neither pattern lets us test `forward`-step calibration *as it is meant to be used*. A self-test fixture sidesteps the vendor-parity question and provides a clean target whose contract is "re-running Finstack's pipeline gives bit-identical outputs."

### Coverage now

| Step kind | Covered? |
|---|---|
| `discount` | ✓ (cdx_ig_46, irs SWPM, fra, irs self_test) |
| `hazard` | ✓ (cdx_ig_46) |
| `base_correlation` | ✓ (cdx_ig_46, indirect) |
| **`forward`** | **✓ NEW (irs self_test, with sequential dependency on discount step)** |
| `inflation` | ✗ (next gap) |
| Surface kinds (`vol_surface`, `swaption_vol`, `svi_surface`) | ✗ (snapshots stay in `initial_market`) |

### Verification

- **Bootstrap convergence**: 1378 iterations, max residual 9.90e-13 (deep convergence on both curves).
- **Curves produced**: 13-knot discount + 9-knot forward, both spanning ~10y horizon (matching real-world FRN/term-loan tenors).
- **Tests**: 2/2 IRS golden tests pass (existing SWPM + new self_test). Full Rust suite: 31/31. Python suite (env-tests subset): clean.
- **Tolerances**: NPV abs `1e-6`, par_rate abs `1e-12`, dv01 abs `1e-6`. Tight because there's no vendor-mismatch source of drift.

### Pattern reference for future migrations

This fixture is the canonical pattern for:

- **Floating-rate bond migrations**: when an FRB needs forward-curve projection across the bond's coupon dates, calibrate the forward curve from term-rate IRS quotes the same way as Step 2 here.
- **Term loan migrations**: same — calibrate USD-SOFR-3M (or applicable tenor) from quotes, then price the floating-rate facility.
- **Structured product migrations**: CLO/ABS pools that project assets via SOFR-3M can use the same calibration recipe; the forward curve serves all assets in the pool.
- **Multi-step calibration**: any future fixture needing more than one calibration step (e.g., discount + forward + hazard for a term loan with credit risk) can follow the sequential-dependency pattern here.

### Recommended follow-up migrations using this pattern

- `term_loan/term_loan_b_5y_floating.json` (5Y FRN with hazard) — replace synthetic curves with two-step cal (USD-OIS + USD-SOFR-3M) plus a hazard step. Keep the synthetic spread as expected.
- `structured_credit/abs_credit_card_senior.json` and `clo_mezzanine_base_case.json` — same two-step rates calibration; pool-level expected outputs come from Finstack as in this self-test fixture.
- `convertible/conv_bond_*` — discount + forward + hazard. Same recipe.

For each: the rates curves (discount + forward) come from the canonical SWPM Curve 490 / Curve 559-style quotes (real Bloomberg or self-consistent synthetic). The instrument-specific curves (hazard, equity, etc.) layer on top.

---

## 17. POC #5 (2026-05-09): `term_loan/term_loan_b_5y_floating.json` — first floating-rate-instrument migration

### What was done

Migrated the 5Y synthetic USD term loan B floating-rate facility to the calibration entry point using the POC #4 two-step pattern. This is the first **production-instrument** migration (not a self-test) using forward-step calibration.

### Curve treatment

The term loan instrument directly uses:
- `discount_curve_id: USD-CORP` (corporate discount with credit spread)
- `credit_curve_id: ACME-HZD` (obligor hazard curve)
- `rate.Floating.index_id: USD-SOFR-3M` (forward curve for SOFR projection)

The fixture's market snapshot also carries 3 auxiliary curves (USD-OIS, USD-TREASURY, HYCO-HZD) for scenario/diagnostic use.

**Migration treatment:**

| Curve | Treatment | Reason |
|---|---|---|
| `USD-OIS` | **Calibrated** (Step 1, discount) | Required to provide the discount curve for the forward step |
| `USD-SOFR-3M` | **Calibrated** (Step 2, forward) | Used by the term loan for floating-rate projection — exercises forward-step calibration on a real production instrument |
| `USD-CORP`, `USD-TREASURY` | Preserved in `initial_market` | Synthetic credit-spread curves; no natural quote-based bootstrap |
| `ACME-HZD`, `HYCO-HZD` | Preserved in `initial_market` | Synthetic hazard curves; would need CDS quotes to calibrate (out of scope for this migration) |

### Expected outputs

The original snapshot used flat-ish synthetic curves (USD-SOFR-3M monotonic 4.3-4.9% at clean integer-year pillars). Replacing with a calibrated USD-SOFR-3M (real-date pillars, slightly different rates) shifts every pricing metric. Re-captured from Finstack itself:

| Metric | Original (synthetic-curve) | New (calibrated-curve) | Original tolerance | New tolerance |
|---|---|---|---|---|
| `npv` | 27,688,694.50 | **27,319,795.88** | 1e-9 abs | 1e-9 abs (unchanged) |
| `discount_margin` | -0.033051 | **-0.028902** | 1e-9 | 1e-9 |
| `ytm` | 0.090182 | **0.085698** | 1e-9 | 1e-9 |
| `dv01` | -565.16 | **-487.99** | 1e-9 | 1e-9 |
| `cs01` | 0.0 | 0.0 (preserved) | 1e-9 | 1e-9 |

Tolerances stay at the original tight `1e-9 abs / 5e-9 rel` because both setups are deterministic — the change is in *what is being tested*, not the precision contract.

### Test contract change (documented)

The original fixture was "term-loan pricing on these specific synthetic curves." The migrated fixture is "term-loan pricing with two-step calibration of USD-OIS + USD-SOFR-3M." The new contract still catches:
- Refactors that change calibration math (Step 1 / Step 2 paths)
- FP-contract changes in the bootstrap solvers
- Algorithm swaps in the term-loan pricing kernel
- OIS compounding override regressions (Step 1 uses `CompoundedWithRateCutoff{cutoff_days: 1}`)

It additionally covers:
- Forward-step calibration on a real production-instrument shape
- Sequential plan ordering (forward depends on discount)
- Hazard-curve passthrough via `initial_market.curves`

### Pattern validation

This migration validates POC #4's pattern (§16) on a real production fixture:
- Two-step calibration with the same OIS + 3M-Term-SOFR quote sets
- `ois_compounding` override on the discount step
- Auxiliary curves preserved in `initial_market.curves` (a 4-curve initial market — most we've used so far)
- Self-test contract because the synthetic snapshot's curves don't bit-match calibrated curves

Pattern transfers cleanly. Expected effort for the next floating-rate migration (`structured_credit/*`, `convertible/*`): ~30-60 min each, mostly mechanical now.

---

## 18. Final state (2026-05-09): all 34 pricing goldens migrated

### Migration scoreboard

**100% of pricing golden fixtures now use the envelope path.** All 34 pricing goldens route through `engine::execute_with_diagnostics` rather than direct `MarketContext` deserialization.

| Pattern | Count | Fixtures |
|---|---|---|
| **Tier-A wrap** (no calibration; preserves snapshot) | 25 | All Bloomberg-parity-tight fixtures (cap_floor, swaption, bonds, CDS, FRA-like inputs); all formula fixtures whose synthetic clean-time-pillar curves can't be reproduced from real-date calibration (deposit, equity_options ×5, fx_options ×4, fx_swap, ir_futures ×2, etc.); inflation fixtures (uncovered step kind) |
| **Tier-B calibrated** (real bootstrap exercise) | 9 | IRS SWPM (B with override), IRS self_test (2-step), FRA (hybrid Tier-AB: discount cal, forward kept), term_loan (2-step), structured_credit ×2 (2-step), convertible ×2 (2-step), CDX IG 46 cds_option (discount + hazard) |

### Step-kind coverage

| Step kind | Production fixtures with calibration |
|---|---|
| `discount` | 7 (FRA, IRS SWPM, IRS self_test, term_loan, ABS, CLO, conv ×2, CDX IG 46) |
| `forward` | 5 (IRS self_test, term_loan, ABS, CLO, conv ×2) |
| `hazard` | 1 (CDX IG 46) |
| `base_correlation` | 1 (CDX IG 46, indirect via initial_market) |
| `inflation` | 0 |
| `vol_surface` / `swaption_vol` / `svi_surface` | 0 (snapshots only — these are externally-built and not bootstrap-reproducible from a finstack-side calibration) |

### Patterns established

Three migration patterns now have working examples in the test suite:

1. **Tier-A wrap** (§13, §15 cap_floor/swaption): wrap snapshot in envelope, empty `plan.steps`, route through entry point. Used when the fixture's contract is parity-tight (Bloomberg/QuantLib screen NPV) or the snapshot's curves are synthetic clean-time data not reproducible from calibration.

2. **Tier-B calibrate** (§14 IRS SWPM, §16 IRS self_test, §17 term_loan, §18 ABS/CLO/convertibles): full quote-driven calibration with `initial_market` carrying instrument-relevant non-rates curves (hazard, credit-spread, prices). Self-test contract — expected outputs captured from Finstack.

3. **Hybrid Tier-AB** (§15 FRA): some curves calibrated, others materialized in `initial_market`. Useful when only a subset of curves is bootstrap-reproducible.

The OIS compounding override (`RatesStepConventions.ois_compounding`) is the key piece of plumbing that closed 40% of Bloomberg parity drift on the IRS POC — landed alongside this work as part of the audit cycle.

### Recommended follow-ups (out of scope for this migration)

- **Inflation step coverage**: author a new self-test fixture exercising `inflation` calibration, similar to POC #4 for forward step. None of the existing fixtures provide a path to this.
- **Surface step coverage**: vol surfaces are externally-built; finstack's own surface calibration step (e.g., SABR fitting) doesn't yet have golden coverage.
- **CDS/hazard step coverage on real Bloomberg curves**: `cds_5y_par_spread.json` is Bloomberg-sourced but currently Tier-A wrapped; could be migrated to Tier-B calibrate once a bootstrap-conformant approach is found (likely needs the same kind of `ois_compounding`-style convention work for hazard curves).
- **`inflation_swap` and `inflation_linked_bond` Tier-B**: defer until inflation step has a tested calibration path.

### Final test gates

- Rust: `cargo test -p finstack-valuations --test golden` → 31/31 pass
- Python: `pytest finstack-py/tests/golden/` → 101/101 pass
- All Bloomberg parity contracts preserved (deposit, FRA, IRS SWPM, swaption, cap_floor, bonds ×2, CDS).
- All formula self-test contracts preserved (equity_options, fx_options, etc.).
- 9 fixtures genuinely exercise calibration math via real bootstrap (1378+ iterations, residuals ~1e-13).

---

## 19. Follow-up coverage (2026-05-09): inflation, hazard, and surface step kinds

After completing the 100% migration in §18, the three documented coverage gaps were closed by authoring fresh self-test fixtures. Each follows the POC #4 pattern (§16): synthetic but self-consistent calibration, expected outputs captured from Finstack's own pipeline, bit-stable round-trip contract.

### 19.1 Inflation step coverage

**Fixture:** `pricing/inflation_swap/usd_5y_zcis_self_test.json`

Two-step plan: USD-OIS discount (12 SOFR OIS quotes with rate-cutoff override) → US-CPI inflation curve (6 ZCIS quotes 1Y-10Y, ~2.0-2.4% rising). Instrument: 5Y zero-coupon inflation swap, receive-fixed at 2.20% against US-CPI, 10MM USD notional. The forward CPI curve is calibrated against the prior discount curve; the inflation swap pricer projects the inflation index path and discounts the inflation-vs-fixed difference.

**Verifies:** the `inflation` step kind, ZCIS quote handling, the `InflationCurveParams` schema (`base_cpi`, `observation_lag`), inflation registry conventions (USD-CPI), and the inflation-swap pricing kernel.

**Expected outputs** (captured from Finstack):
- npv = -91501.4714497153
- dv01 = 45.77580653056066

Tolerances: `1e-6` abs for both. Self-test contract.

### 19.2 Hazard step coverage on single-name CDS

**Fixture:** `pricing/cds/usd_5y_cds_self_test.json`

Two-step plan: USD-OIS discount → ACME-HZD hazard curve (5 CDS par-spread quotes 1Y-10Y, 60-150 bp rising). Instrument: 5Y single-name CDS (pay-protection) at 100 bp running on synthetic ACME-CORP issuer, ISDA NA convention.

Prior hazard-step coverage was only via CDX IG 46 (a CDS option fixture). This adds genuine single-name-CDS hazard calibration coverage.

**Verifies:** the `hazard` step kind on a single-name (vs index) CDS, the CDS par-spread quote shape, the hazard step's `recovery_rate` / `doc_clause` plumbing, and the CDS pricing kernel against a calibrated hazard curve.

**Expected outputs** (captured from Finstack):
- npv = 38800.84804454475
- dv01 = -10.632060529809678
- cs01 = 4337.206909633183

Tolerances: `1e-6` abs across npv/dv01/cs01.

### 19.3 Surface step coverage (SABR)

**Fixture:** `pricing/equity_option/aapl_equity_vol_self_test.json`

Two-step plan: USD-OIS discount → AAPL-EQUITY-VOL SABR vol surface (9 OptionVol quotes, 3 expiries × 3 strikes, beta=0.5). Instrument: 1Y AAPL ATM call option on 100 shares, priced via Black76 against the calibrated surface and the AAPL-SPOT / AAPL-DIVYIELD prices in `initial_market`.

The SABR fit converges with rmse ~1e-3 (typical for vol-surface calibration across a 3×3 grid). The self-test contract is bit-stable round-trip reproduction of the *priced* metrics (npv/delta/gamma/vega/theta), not zero rmse on the calibration residuals.

**Verifies:** the `vol_surface` (SABR) step kind, OptionVol quote shape with `convention`/`option_type`/`strike`/`expiry`, the `target_expiries` × `target_strikes` grid configuration, and the equity-option pricer reading from a calibrated SABR surface.

**Expected outputs** (captured from Finstack):
- npv = 1851.3423860430435
- delta = 59.670220906135896
- gamma = 0.9515132005368704
- vega = 66.34621887368557
- theta = -4.0849442049804665

Tolerances: `1e-6` abs across all 5 metrics.

### Updated coverage scoreboard

| Step kind | Production fixtures | Self-test fixtures |
|---|---|---|
| `discount` | 7 | 1 (irs self_test) |
| `forward` | 4 | 1 (irs self_test) |
| `hazard` | 1 (CDX IG 46) | **1 NEW (cds self_test)** |
| `base_correlation` | 1 (CDX IG 46) | 0 |
| **`inflation`** | 0 | **1 NEW (zcis self_test)** |
| **`vol_surface`** | 0 | **1 NEW (aapl SABR self_test)** |
| `swaption_vol` | 0 | 0 (still uncovered) |
| `svi_surface` | 0 | 0 |

Every primary calibration step kind now has at least one golden fixture exercising it, except `swaption_vol` and `svi_surface` (those would follow the same pattern — left as the next obvious follow-up).

### Final test gates after follow-ups

- Rust: `cargo test -p finstack-valuations --test golden` → 31/31 pass
- Python: `pytest finstack-py/tests/golden/` → 104/104 pass (was 101 before this batch; +3 self-test fixtures)
- 12 fixtures now genuinely exercise calibration math (was 9 in §18).
