# Quant-Audit Remediation — Deferred Items

**Date:** 2026-04-21
**Source roadmap:** `docs/superpowers/plans/2026-04-19-quant-audit-remediation-roadmap.md` (local, gitignored — per-session planning artifact)
**Status:** Active
**Cross-references:**
  - `INVARIANTS.md` (repo root) — cross-crate rules the remaining PRs must respect
  - `docs/REFERENCES.md` — canonical references used for audit-driven fixes

This document lists the quant-audit findings that were **not** closed in
the initial remediation sprint (PRs 0, 1, 2, 3, 4, 5, 6, 10, 12, 13, 14,
15, 16), along with a concrete rationale for why each was deferred and
what the prerequisites are for picking it up.

The "Cumulative progress" section below summarizes what's merged. The
"Deferred items" section is the actionable part.

---

## Cumulative progress

| Status | Count | Findings |
|--------|-------|----------|
| **Merged to master** | 16 | C1, C2, C3, C4, C5, C6, C7, C8, C9 (partial), P1 #18, P1 #20, P1 #23 (partial), P2 #26, P2 #30, P3 #31, P3 #33 |
| Deferred | 19 | C9 (co-terminal repricer), C10–C14, C15–C20, P1 #15, P1 #16, P1 #17, P1 #19, P1 #21, P1 #22, P2 #24, P2 #25, P2 #27, P2 #28, P2 #29, P3 #32, P3 #34, P3 #35 |

16/35 audit findings materially closed in-session. Remaining 19 deferred
with specific reasons below.

---

## Deferred items

### Tier P0 — ship-blockers still open

#### C9 (part 3) — True co-terminal swaption repricer for LMM validation

- **Original roadmap target:** PR 6 third sub-item.
- **What landed:** strict-mode escalation for Rebonato discriminant + PCA
  variance-loss reporting.
- **What's missing:** the residual calculation currently uses
  `rebonato_swaption_vol` both for stripping and for post-solve
  reporting, so "how well did we fit" is measured against the same
  approximation we used to strip. The audit requires an independent
  true-swaption repricer (Brigo–Mercurio §6.14 / Andersen-Piterbarg
  Vol II Ch. 18) to validate calibration quality without circularity.
- **Why deferred:** implementing a production-grade LMM co-terminal
  swaption re-pricer involves either nested Monte Carlo (expensive,
  noisy) or a Longstaff-Schwartz-style conditional expectation (needs
  basis-function tuning) — non-trivial. Useful but not a bug fix.
- **Prerequisites to start:** none; can land after the other P0 items.
- **Estimated effort:** 2–3 days.

#### C10–C14 — ISDA SIMM aggregation + XVA CSA split + FRTB scenario max

- **Original roadmap target:** PR 7 (bundled ~2 weeks per roadmap).
- **Why deferred:** the correctness proof is **not code-first**. It
  requires ISDA's public SIMM v2.6 unit-test vectors as golden files
  under `finstack/margin/tests/simm/isda_v2_6/`. Those vectors do not
  currently exist in the repo, and fabricating them during a code
  session would be worse than not having them — a rewrite of
  `calculate_from_sensitivities` could pass a superficial smoke test
  while silently producing non-compliant IM numbers, which is exactly
  the failure mode the audit was meant to expose.
- **Prerequisites to start:**
  1. Obtain ISDA SIMM v2.6 public Methodology & Unit Test Package.
  2. Commit the JSON fixtures under `finstack/margin/tests/simm/isda_v2_6/`.
  3. Write a parity runner that exercises each fixture end-to-end.
  4. **Only then** begin rewriting the SIMM aggregation head in
     `finstack/margin/src/calculators/im/simm.rs`.
- **Estimated effort:** ~2 weeks after prerequisites.

#### C15–C20 — statements sign-convention / TTM / PIT refactor

- **Original roadmap target:** PR 7b (bundled ~1.5 weeks per roadmap).
- **Why deferred:** this is a sweeping public-API refactor of
  `FinancialModelSpec` that ripples through ≥15 downstream callers in
  `finstack-statements-analytics` (analysis/reports, scorecards,
  covenants, DCF valuation, credit context, backtesting), the Python
  bindings, and the WASM bindings. Concentrated as a single session
  commit, it's not reviewable; spread across sessions, each
  downstream rewrite needs its own test-driven update.
- **Specifically still open:**
  - C15 — Decimal invariant violation in `statements` evaluator
    (`Vec<Option<f64>>` at the storage level).
  - C16 — three divergent "Debt/EBITDA" formulas in reports vs. checks
    vs. DCF.
  - C17 — sign-convention contradictions between CashReconciliation,
    DepreciationReconciliation, and RetainedEarningsReconciliation.
  - C18 — `ttm()` silently returns truncated sums when fewer than
    `window` periods are available.
  - C19 — no `as_of` PIT filter anywhere in the evaluator.
  - C20 — implied-interest reconciliation uses EOP balance instead of
    `(B_{t-1} + B_t)/2` averaged balance.
- **Prerequisites to start:**
  1. Design document for the `SignConventionPolicy` + `AsOfPolicy`
     enums and the migration path for the 15+ downstream callers.
  2. Multi-PR decomposition: one PR per sub-item (C15 through C20),
     not a single mega-PR.
  3. Breaking-change migration plan for the next major version tag
     (roadmap says `v0.5.0`).
- **Estimated effort:** ~1.5 weeks across 6 dependent PRs.

### Tier P1 — high-value open items

#### P1 #15 — Student-t Laguerre quadrature on demand

- **Original roadmap target:** PR 8 first sub-item.
- **Why deferred:** bundled with P1 #16 (below), which requires a
  sweeping trait-level change. #15 alone is tractable but landing it
  without #16 leaves the hardcoded 10-node table visible from the
  same module as the broken single-sector copula.
- **Prerequisites to start:** Golub–Welsch generator for
  generalized Laguerre; this should live in
  `finstack/core/src/math/quadrature/golub_welsch.rs` (new module).
- **Estimated effort:** 1–2 days.

#### P1 #16 — Multi-factor copula sector awareness

- **Original roadmap target:** PR 8 second sub-item.
- **Why deferred:** requires extending the `Copula` trait with a
  `conditional_default_prob_with_sector(threshold, factors, ρ, sector_idx)`
  method; every implementor (Gaussian, Student-t, MultiFactor, RFL)
  needs the new method; every tranche-pricing workflow that consumes
  copulas needs the sector index threaded through. Multi-day trait
  migration.
- **Prerequisites to start:** decide on the trait signature (add the
  method as non-default, or default to the 1-factor method with a
  `CopulaError::SectorIndexRequired` fallback?). Design doc
  recommended before code.
- **Estimated effort:** 2–3 days.

#### P1 #17 — Adjusted Net Debt bridge in statements-analytics

- **Original roadmap target:** PR 9.
- **Why deferred:** PR 9 depends on PR 7b's `AdjustedNetDebt` scaffold
  existing; PR 7b itself is deferred (see C15–C20 above). Cannot
  proceed until PR 7b (even in minimal-viable form) lands first.
- **Estimated effort:** 3 days after PR 7b completes.

#### P1 #19 — `r_eff` two-clock plumbing across barrier / asian / fx pricers

- **Original roadmap target:** PR 11.
- **Why deferred:** the `r_eff = -ln(DF)/t_vol` pattern spans **12+
  call sites** across `finstack/valuations/src/instruments/exotics/`
  and `.../fx/`, and proper remediation requires threading **both**
  `t_disc` and `t_vol` through each pricer's entry point with
  per-pricer day-count audit. Without per-pricer golden values to
  verify the drift correction, a blanket replacement could silently
  shift prices in the wrong direction.
- **Prerequisites to start:**
  1. Assemble per-pricer QuantLib-Python golden values at varying
     day-count combinations.
  2. Introduce a `TwoClockParams { t_disc, t_vol, df }` helper struct
     in `finstack/valuations/src/instruments/common/helpers/`.
  3. Migrate pricers one at a time; each pricer gets its own PR.
- **Estimated effort:** 2–3 days per pricer × ~6 pricers ≈ 2 weeks.

#### P1 #21 — ISDA 2021 overnight observation-shift semantics

- **Original roadmap target:** PR 13 first sub-item.
- **What landed:** only the third sub-item (scenarios `try_compose`).
- **What's missing:** `finstack/cashflows/src/builder/emission/coupons.rs`
  `sample_overnight_rates` currently shifts observation indices
  *within* the accrual window; ISDA 2021 Supp. 70 §7.1(g) and ARRC
  2020 §2 define observation shift as moving the **window itself**
  before the accrual period. Difference is 2–5 basis points on
  SOFR/SONIA in normal regimes, up to 10+ bp at rate-move boundaries.
- **Prerequisites to start:**
  1. SOFR golden fixture from ARRC 2020 conventions at 2 BD lookback.
  2. SONIA golden fixture from BoE SONIA Compounded Index Guide at
     5 BD.
- **Estimated effort:** 1.5 days.

#### P1 #22 — GARCH MLE multi-start + full Hessian

- **Original roadmap target:** PR 13 second sub-item.
- **What's missing:** `finstack/analytics/src/timeseries/optimizer.rs`
  uses a single Nelder-Mead simplex; `garch.rs` uses diagonal
  Hessian inversion (understating α / β standard errors near the
  stationarity frontier).
- **Prerequisites to start:** `nalgebra::Cholesky` path for the full
  Hessian inverse with Tikhonov fallback; Halton-perturbed 4-restart
  wrapper (can potentially reuse the PR 4 `multi_start` helper from
  `valuations/calibration/solver` by promoting it to a core crate).
- **Estimated effort:** 1 day.

#### P1 #23 — ISDA 2021 / GARCH bundled items

- These are the two sub-items of PR 13 that weren't closed. See P1
  #21 and P1 #22 above; only scenarios `try_compose` landed. Tracked
  as "P1 #23 (partial)" in the roadmap.

### Tier P2 — quality & compliance

#### P2 #24 — Registry-backed FRTB parameters

- **Original roadmap target:** PR 14 first sub-item.
- **What's missing:** migrate
  `finstack/margin/src/regulatory/frtb/params/{girr,csr,equity,commodity,fx}.rs`
  from hardcoded Rust constants to JSON overlays loaded via
  `embedded_registry`, each pinned to a BCBS publication date
  (d457 vs d554). Currently the workspace cannot side-by-side
  regression test two FRTB revisions.
- **Prerequisites to start:** confirmed JSON schema for each of the
  5 parameter files; CI gate that loads each overlay and snapshots
  the derived calculator for diff-detection.
- **Estimated effort:** 2 days.

#### P2 #25 — Correlation matrix PSD check at registry load

- **Original roadmap target:** PR 14 second sub-item.
- **What's missing:** no runtime PSD validation is performed on the
  hand-typed correlation matrices shipped with SIMM (17×17 commodity)
  or CSR (sector correlations). A non-PSD matrix would silently
  propagate through the IM calculation and produce negative-variance
  exposures where the sqrt-of-sum goes complex or clamps.
- **Prerequisites to start:** promote
  `finstack/core/src/math/linalg.rs::cholesky_correlation`
  (which performs pivoted PSD-with-floor) to a workspace utility that
  registry loaders call at construction; emit
  `RegistryError::NonPsdCorrelationMatrix` with offending
  eigenvalues.
- **Estimated effort:** 2 days.

#### P2 #27 — Brinson-Fachler attribution in portfolio

- **Original roadmap target:** PR 15 first sub-item.
- **What's missing:** `finstack/portfolio/src/attribution.rs` implements
  a factor-based attribution but not the classical Brinson three-way
  decomposition (Allocation, Selection, Interaction + Currency) with
  Carino linking across multi-period horizons. The audit explicitly
  called this out as a deliverability gap for benchmark-relative
  reporting.
- **Prerequisites to start:** Grinold–Kahn §17 reference
  implementation + Carino 1999 smoothing formula; fixture for
  attribution based on a 2-period 3-sector illustrative portfolio.
- **Estimated effort:** 3 days.

#### P2 #28 — SVI slice interpolation in total variance `w(K, T)`

- **Original roadmap target:** PR 15 second sub-item.
- **What's missing:** `finstack/valuations/src/calibration/targets/svi.rs`
  interpolates implied vol linearly in `T` between slices, which
  permits calendar-spread arbitrage. Gatheral's recipe is to
  interpolate `w(K, T) = σ²·T` and verify calendar + butterfly
  no-arbitrage post-fit.
- **Prerequisites to start:** invoke existing
  `SurfaceValidator::{validate_calendar_spread, validate_butterfly_spread}`
  post-build in `step_runtime.rs` SVI path.
- **Estimated effort:** 1 day.

#### P2 #29 — Vanna-Volga FX + digital replication

- **Original roadmap target:** PR 15 third sub-item.
- **What's missing:** the `FxBarrierVannaVolga` module exists at
  `finstack/valuations/src/instruments/fx/fx_barrier_option/vanna_volga.rs`
  but is not wired into the pricer registry; FX digitals use Black-76
  without the volatility smile correction (10–30 bp miss per option
  on typical FX vols).
- **Prerequisites to start:** golden-value harness (Bloomberg/Vega
  FX Volatility Smile comparison package) for both pricers.
- **Estimated effort:** 3 days.

### Tier P3 — hygiene

#### P3 #32 — `FactorBumpUnit` enum for delta_engine

- **Original roadmap target:** PR 16 second sub-item.
- **What's missing:** `finstack/valuations/src/factor_model/sensitivity/delta_engine.rs`
  treats the per-factor `bump_size` differently depending on
  `MarketFactor` variant (%, bp, absolute) without a type-level
  guarantee. The audit called out that equivalent numeric inputs can
  mean very different things across factors.
- **Prerequisites to start:** introduce
  `enum FactorBumpUnit { Absolute, BasisPoint, Percent, Multiplier }`
  and thread it alongside `bump_size` in `MarketFactor`.
- **Estimated effort:** 1 day.

#### P3 #34 — Replace Jacobi eigensolver in nearest_correlation

- **Original roadmap target:** PR 16 fourth sub-item.
- **What's missing:** `finstack/valuations/src/correlation/nearest_correlation.rs`
  uses a hand-rolled Jacobi eigensolver with `100·n²` max sweeps. For
  `n > 40` this is `O(n⁵)` worst case and dominates Higham projection
  wall-time.
- **Prerequisites to start:** replace with `nalgebra::SymmetricEigen`
  (already a workspace dependency).
- **Estimated effort:** 0.5 day.

#### P3 #35 — Consolidation of model-version strings and percent-equality helper

- **Original roadmap target:** PR 16 fifth sub-item.
- **What's missing:** model-version strings ("ISDA Standard Model
  v1.8.2", "Student-t Copula Calibration v1.0", …) are hardcoded in
  individual files; a central `finstack_core::versions` module would
  surface drift at review time. Percent-equality logic likewise
  duplicated (base-correlation vs knots use different tolerances).
- **Prerequisites to start:** none, pure refactor.
- **Estimated effort:** 1 day.

---

## Deferred totals by tier

| Tier | Deferred findings | Estimated effort |
|------|-------------------|------------------|
| P0 (ship-blockers) | C9 part 3, C10–C14, C15–C20 | ~4 weeks |
| P1 | #15, #16, #17, #19, #21, #22 | ~3 weeks |
| P2 | #24, #25, #27, #28, #29 | ~2 weeks |
| P3 | #32, #34, #35 | ~2 days |

**Total remaining effort:** approximately 9 engineer-weeks across 19
findings.

---

## Recommendations for next sprints

### Sprint priorities

1. **C15–C20 statements refactor** — highest downstream impact
   (blocks PR 9 = P1 #17). Start with C18 (TTM `min_periods`) and C19
   (PIT filter) as independent PRs before tackling the sign-convention
   refactor.
2. **C10–C14 SIMM rewrite** — regulatory-critical. Sequence:
   (a) commit ISDA v2.6 fixtures, (b) write parity runner, (c) fix
   aggregation head. Do not skip step (a).
3. **P1 #19 r_eff plumbing** — measurable pricing impact on
   cross-currency books; one-pricer-at-a-time migration plan.
4. **P2 / P3 items** — independently landable; good for filling gaps
   between larger pieces.

### Process notes

* Each remaining PR should follow the same discipline as the 13
  landed: failing regression test committed with the fix, citation of
  the audit finding ID in both the test and the commit message, clean
  clippy under `-D warnings`, and a scope-note in the commit message
  listing what was explicitly deferred further.
* The 13 merged PR branches (`feat/quant-audit-pr0-hagan-west`
  through `-pr16-hygiene`) are retained on the local repository for
  cherry-picking reference — they can be pruned with
  `git branch -d feat/quant-audit-*` once no follow-ups need them.
* `INVARIANTS.md` at the repo root now documents the cross-crate
  rules that the remaining PRs must respect; read it before starting
  a new sub-item.

---

## Cross-reference index

For each audit finding ID, the table below points to either the
commit that closed it or the section in this document that explains
the deferral.

| Finding | Status | Commit or doc-section |
|---------|--------|-----------------------|
| C1 | merged | `99ae053b6` PR 0 |
| C2 | merged | `d957bd0ba` PR 1 |
| C3 | merged | `d957bd0ba` PR 1 |
| C4 | merged | `6d51a7b06` PR 2 |
| C5 | merged | `5a66364a2` PR 3 |
| C6 | merged | `5a66364a2` PR 3 |
| C7 | merged | `6d51a7b06` PR 2 |
| C8 | merged | `a149898ff` PR 5 |
| C9 (partial) | merged | `10b273527` PR 6 |
| C9 (co-terminal repricer) | deferred | §C9 part 3 |
| C10–C14 | deferred | §ISDA SIMM |
| C15–C20 | deferred | §statements refactor |
| P1 #15 | deferred | §Student-t quadrature |
| P1 #16 | deferred | §Multi-factor copula |
| P1 #17 | deferred | §Adjusted Net Debt |
| P1 #18 | merged | `59b9e0b51` PR 10 |
| P1 #19 | deferred | §r_eff plumbing |
| P1 #20 | merged | `4558acc23` PR 12 |
| P1 #21 | deferred | §ISDA 2021 overnight |
| P1 #22 | deferred | §GARCH MLE |
| P1 #23 (partial) | merged | `b07f3d75c` PR 13 |
| P1 #23 (overnight, GARCH) | deferred | §P1 #21, §P1 #22 |
| P2 #24 | deferred | §FRTB registry |
| P2 #25 | deferred | §PSD check |
| P2 #26 | merged | `299b44be1` PR 14 |
| P2 #27 | deferred | §Brinson |
| P2 #28 | deferred | §SVI total variance |
| P2 #29 | deferred | §Vanna-Volga FX |
| P2 #30 | merged | `698ae9ef4` PR 15 |
| P3 #31 | merged | `31f25a621` PR 16 |
| P3 #32 | deferred | §FactorBumpUnit |
| P3 #33 | merged | `31f25a621` PR 16 |
| P3 #34 | deferred | §Jacobi eigensolver |
| P3 #35 | deferred | §Consolidation |
