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
| **Merged to master** | 24 | C1, C2, C3, C4, C5, C6, C7, C8, C9 (partial), P1 #15, P1 #16, P1 #18, P1 #20, P1 #21, P1 #22, P1 #23 (partial), P2 #25, P2 #26, P2 #28, P2 #30, P3 #31, P3 #33, **N2** |
| Deferred | 11 | C9 (co-terminal repricer), C10–C14, C15–C20, P1 #17, P1 #19, P2 #24, P2 #27, P2 #29, P3 #32, P3 #34, P3 #35 |
| Re-classified | 1 | **N1** (see §New findings) |

24/35 + 1 new audit findings materially closed. Remaining 11 deferred
with specific reasons below.

### Recently closed — follow-up session (2026-04-21 cont.)

Five more items landed in the follow-up session after the initial
deep-audit-driven batch:

- **P2 #25 commodity matrix** — migrated the 17×17 commodity inter-
  bucket correlation matrix from a hardcoded `const` free function to
  `SimmParams::commodity_inter_bucket_correlations` and routed it
  through `validate_simm_correlations_psd`, so the same PSD guard now
  covers every registry-loaded SIMM correlation matrix. New tests
  assert PSD of the embedded matrix and rejection of both non-PSD and
  wrong-shape inputs.
- **P1 #21 Lookback variant** — new
  `sample_overnight_rates_with_lookback` in `cashflows` looks up each
  rate via `add_business_days(-lookback_bd)` per ARRC 2020 §2, pairing
  pre-accrual-window rates with accrual-window weights. The in-
  dispatcher index-rewriting that could not access rates from before
  `accrual_start` is removed. Regression test on a rising forward
  curve asserts the lookback coupon rate is strictly below the in-
  arrears rate by more than 1 bp.
- **P1 #22 GARCH MLE** — `invert_hessian_diag` now routes through a
  Cholesky-first, Tikhonov-regularized-fallback `invert_hessian_full`
  so the Cramér-Rao diagonal stays well-defined at the stationarity
  frontier. `NelderMead::minimize_multi_start` adds Halton-perturbed
  multi-start; `FitConfig::num_restarts` (default 4) and
  `FitConfig::perturbation_scale` wire it through to every
  `fit_garch_mle` caller. Regression tests cover (1) the tilted
  double-well escape case, (2) Tikhonov fallback on a rank-1
  singular Hessian, (3) off-diagonal Hessian coupling in the variance
  estimate.
- **P2 #28 SVI total-variance interpolation** — replaced the
  parameter-space linear interpolation in `calibration/targets/svi.rs`
  with Gatheral total-variance interpolation `w(k, T) = (1 − τ)·w(k, T₁)
  + τ·w(k, T₂)` which preserves calendar-spread monotonicity by
  construction. Post-build the grid is handed to
  `SurfaceValidator::{validate_calendar_spread, validate_butterfly_spread}`
  with `lenient_arbitrage = true` so residual arbitrage surfaces via
  tracing. New tests anchor the Gatheral identity `σ = √(w/T)` and
  verify calendar monotonicity across interpolated expiries.
- **P1 #15 Golub-Welsch quadrature** — new
  `finstack_core::math::GaussLaguerreQuadrature` computes Gauss-Laguerre
  nodes and weights at runtime via spectral decomposition of the Jacobi
  matrix (any `n ≥ 1`, `O(n²)` startup). `StudentTCopula` now consumes
  this instead of the hardcoded 10-node table, so
  `with_quadrature_order(n > 10)` actually delivers an `n`-node rule up
  to a new `MAX_LAGUERRE_ORDER = 64` ceiling. Regression tests cover
  (1) the canonical 10-node table match, (2) moment-exactness through
  degree `2n − 1`, (3) low-order exactness boundary at `n = 2`, and
  (4) the cap removal via `test_with_quadrature_order_uses_requested_order`.
- **P1 #16 copula sector awareness** — extended the `Copula` trait
  with a default-implemented
  `conditional_default_prob_with_sector(threshold, factors, ρ,
  sector_idx)` method. Sector-unaware copulas (Gaussian, Student-t,
  RFL) inherit the default (sector_idx ignored); `MultiFactorCopula`
  overrides so `sector_idx = 0` drops the sector loading — the cross-
  sector case a pair of names in different sectors should see. The
  legacy `conditional_default_prob` path forwards to `sector_idx = 1`
  so existing pricing routines are unchanged. The K > 2 multi-sector
  extension remains follow-up work; this slice delivers the trait
  scaffolding and the 2-factor override.

### Recently closed (session 2026-04-21)

Three items identified in the 2026-04-21 deep-audit read as
"should-not-defer-further" landed alongside a refreshed audit:

- **P2 #25 — PSD check at SIMM registry load.** `margin/registry/mod.rs`
  now densifies `risk_class_correlations` (6×6), `ir_tenor_correlations`
  (n×n) and `cq_inter_bucket_correlations` (9×9) and routes each
  through `finstack_core::math::linalg::validate_correlation_matrix`.
  A single off-diagonal typo that would previously propagate through
  `K_b = sqrt(Σ WSᵢ² + Σ ρᵢⱼ WSᵢ WSⱼ)` as a clamped/complex sqrt now
  fails registry load with a typed `Error::Validation` naming the
  offending matrix.
- **P1 #21 — ISDA 2021 observation-shift semantics.**
  `cashflows/src/builder/emission/coupons.rs::observation_window` moves
  both endpoints of the observation window back by `shift_days`
  business days before calling `sample_overnight_rates`, so rates AND
  per-day weights come from the shifted window per ISDA 2021
  Supp. 70 §7.1(g). The post-hoc index rewriting in
  `compute_overnight_rate::CompoundedWithObservationShift` is removed
  (it could not access pre-accrual rates). Regression test
  `test_overnight_observation_shift_samples_pre_accrual_window` in
  `cashflows/tests/cashflows/builder/floating_rate.rs` verifies that on
  a rising forward curve the shifted coupon rate is strictly below the
  in-arrears coupon rate. The `CompoundedWithLookback` variant still
  applies its shift inside the accrual window and is tracked as a
  follow-up (ARRC 2020 SOFR 2 BD lookback).
- **N2 — SA-CCR supervisory-delta sign / direction coherence.** New
  `SaCcrTrade::validate()` method enforces BCBS 279 ¶104–112 sign
  discipline on every trade ingested by `SaCcrEngine::calculate_ead`.
  Linear trades must have `supervisory_delta = ±1` matching
  `direction`; options must have sign-consistent delta per
  `option_type` (CallLong/PutShort positive; CallShort/PutLong
  negative). An upstream misconfigured trade (e.g. long linear passed
  with `supervisory_delta = -1`) now surfaces at the engine boundary
  rather than silently reversing the add-on contribution.

### New findings

- **N1 — SIMM FX delta aggregation shape (re-classified).** The
  2026-04-21 agent-led audit flagged FX concentration as applied
  pool-wide; on a fresh read of
  `margin/src/calculators/im/simm.rs:993-1010` the per-currency
  concentration factor IS correctly applied per-FX-risk-factor. The
  remaining discrepancy is the **aggregation form**: current code
  reduces to `|Σ WS_k · CF_k|` (implicit ρ = 1), whereas ISDA SIMM
  v2.6 §3.6 specifies `K_FX = sqrt(Σ WS_k_conc² + Σ_{k≠l} 0.5 · WS_k_conc · WS_l_conc)`.
  Fixing this requires an FX intra-bucket correlation parameter in
  `SimmParams` and is part of the **C10–C14 SIMM rewrite scope** — it
  cannot be validated without ISDA v2.6 public test vectors. Not an
  independent item; rolled into C10–C14.

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

#### P1 #15 — Student-t Laguerre quadrature on demand — ✅ CLOSED (2026-04-21 follow-up)

- **Original roadmap target:** PR 8 first sub-item.
- **Closed in:** 2026-04-21 follow-up. Implemented at
  `finstack/core/src/math/integration.rs::GaussLaguerreQuadrature`
  (Golub-Welsch via `nalgebra::SymmetricEigen`), not the originally
  proposed `math/quadrature/golub_welsch.rs` sub-directory — a single
  file alongside the existing `GaussHermiteQuadrature` keeps the
  quadrature APIs co-located.

#### P1 #16 — Multi-factor copula sector awareness — ✅ CLOSED (2026-04-21 follow-up)

- **Original roadmap target:** PR 8 second sub-item.
- **Closed in:** 2026-04-21 follow-up. `Copula` trait gained a default-
  implemented `conditional_default_prob_with_sector` method;
  `MultiFactorCopula` overrides it so `sector_idx = 0` drops the sector
  loading (cross-sector correlation reduces to `β_G²`). The legacy
  `conditional_default_prob` path is unchanged (forwards to
  `sector_idx = 1`). Multi-sector (K > 2) generalization remains a
  follow-up: `MultiFactorCopula` still hard-caps at 2 factors (global
  + one sector) and a true `K`-sector copula with
  `factor_realization = [Z_G, Z_S1, …, Z_SK]` requires a new
  constructor family plus integrate_fn plumbing. Effort estimate for
  the multi-sector extension: ~2 days, blocked on design consensus
  around the sector-factor quadrature approach (nested Gauss-Hermite
  vs. Monte Carlo).

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

#### P1 #21 — ISDA 2021 overnight observation-shift semantics — ✅ CLOSED (2026-04-21 session + follow-up)

- **Original roadmap target:** PR 13 first sub-item.
- **Closed in:** 2026-04-21 session (`CompoundedWithObservationShift`
  via `observation_window` helper in `coupons.rs`) plus 2026-04-21
  follow-up (`CompoundedWithLookback` via
  `sample_overnight_rates_with_lookback` in `coupons.rs`, looking up
  each rate at `date.add_business_days(-lookback_bd)` per ARRC 2020
  §2). Both variants now carry regression tests on a rising forward
  curve that assert shifted/lookback coupon rates are strictly below
  the in-arrears rate by more than 1 bp, proving the observation
  window genuinely shifts.

#### P1 #22 — GARCH MLE multi-start + full Hessian — ✅ CLOSED (2026-04-21 follow-up)

- **Original roadmap target:** PR 13 second sub-item.
- **Closed in:** 2026-04-21 follow-up.
  `optimizer::invert_hessian_full` implements Cholesky first with
  Tikhonov-regularized `(H + λI)^{-1}` fallback scaled to the
  Frobenius norm; `invert_hessian_diag` delegates to it so the
  Cramér-Rao diagonal stays well-defined near the stationarity
  frontier.
  `optimizer::NelderMead::minimize_multi_start` drives a Halton-
  perturbed 4-restart multi-start (no RNG, bit-deterministic);
  `FitConfig::num_restarts` / `FitConfig::perturbation_scale` wire
  the behaviour into every GARCH family via `fit_garch_mle`.
  Regression tests cover escape-from-local-minimum, Tikhonov fallback
  on a rank-1 singular Hessian, and the off-diagonal contribution to
  the diagonal of `H^{-1}`.

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

#### P2 #25 — Correlation matrix PSD check at registry load — ✅ CLOSED (2026-04-21 session + follow-up)

- **Original roadmap target:** PR 14 second sub-item.
- **Closed in:** 2026-04-21 session for the three registry-loaded
  matrices (cross-risk-class 6×6, IR tenor n×n, CQ inter-bucket 9×9),
  and 2026-04-21 follow-up for the commodity 17×17 matrix — promoted
  from a hardcoded `const` free function to
  `SimmParams::commodity_inter_bucket_correlations` and routed through
  `validate_simm_correlations_psd`. The lone remaining follow-up is
  the CSR sector correlation matrix (currently unsourced in the
  registry); once it is wired in, extending the validator is a copy-
  paste of the dense-matrix branch.

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

#### P2 #28 — SVI slice interpolation in total variance `w(K, T)` — ✅ CLOSED (2026-04-21 follow-up)

- **Original roadmap target:** PR 15 second sub-item.
- **Closed in:** 2026-04-21 follow-up.
  `calibration/targets/svi.rs::interpolate_svi_vol` now implements
  Gatheral total-variance interpolation
  `w(k, T) = (1 − τ)·w(k, T₁) + τ·w(k, T₂)` which preserves calendar
  monotonicity of `w` by construction. Post-build the grid is handed
  to `SurfaceValidator::{validate_calendar_spread,
  validate_butterfly_spread}` with `lenient_arbitrage = true` so
  residual arbitrage is logged via tracing rather than failing the
  calibration pipeline. New tests anchor the `σ = √(w/T)` identity
  across interpolated expiries.

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
| P1 | #17, #19 | ~2.5 weeks |
| P2 | #24, #27, #29 | ~1.5 weeks |
| P3 | #32, #34, #35 | ~2 days |

**Total remaining effort:** approximately 8 engineer-weeks across 11
findings (was 9 weeks / 19 findings before the 2026-04-21 follow-up
session).

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
| P1 #15 | merged (2026-04-21 follow-up) | `GaussLaguerreQuadrature` in `finstack-core::math::integration`; Student-t copula migrated to runtime Golub-Welsch (`compute_gamma_quadrature`) with `MAX_LAGUERRE_ORDER = 64`. |
| P1 #16 | merged (2026-04-21 follow-up) | `Copula::conditional_default_prob_with_sector` with default impl + `MultiFactorCopula` override for `sector_idx = 0` → cross-sector. K > 2 multi-sector support still follow-up. |
| P1 #17 | deferred | §Adjusted Net Debt |
| P1 #18 | merged | `59b9e0b51` PR 10 |
| P1 #19 | deferred | §r_eff plumbing |
| P1 #20 | merged | `4558acc23` PR 12 |
| P1 #21 | merged (2026-04-21 session + follow-up) | `observation_window` in coupons.rs (ObservationShift); `sample_overnight_rates_with_lookback` (Lookback, follow-up); two regression tests on rising forward curves. |
| P1 #22 | merged (2026-04-21 follow-up) | `invert_hessian_full` with Cholesky + Tikhonov fallback; `NelderMead::minimize_multi_start` with Halton perturbation; `FitConfig::num_restarts = 4` default. |
| P1 #23 (partial) | merged | `b07f3d75c` PR 13 |
| P1 #23 (overnight, GARCH) | deferred | §P1 #21, §P1 #22 |
| P2 #24 | deferred | §FRTB registry |
| P2 #25 | merged (2026-04-21 session + follow-up) | 3×3 registry-load PSD check (initial session); commodity 17×17 matrix promoted to `SimmParams::commodity_inter_bucket_correlations` (follow-up). Remaining follow-up: CSR sector matrix same treatment. |
| P2 #26 | merged | `299b44be1` PR 14 |
| P2 #27 | deferred | §Brinson |
| P2 #28 | merged (2026-04-21 follow-up) | Gatheral total-variance interpolation in `calibration/targets/svi.rs::interpolate_svi_vol`; post-build `SurfaceValidator::{validate_calendar_spread, validate_butterfly_spread}` wired with `lenient_arbitrage = true`. |
| P2 #29 | deferred | §Vanna-Volga FX |
| P2 #30 | merged | `698ae9ef4` PR 15 |
| P3 #31 | merged | `31f25a621` PR 16 |
| P3 #32 | deferred | §FactorBumpUnit |
| P3 #33 | merged | `31f25a621` PR 16 |
| P3 #34 | deferred | §Jacobi eigensolver |
| P3 #35 | deferred | §Consolidation |
| N1 (new) | re-classified into C10–C14 | §New findings — FX aggregation form |
| N2 (new) | merged (2026-04-21 session) | `SaCcrTrade::validate` in `margin/src/regulatory/sa_ccr/types.rs`; tests in `sa_ccr::types::validate_tests::*` and `sa_ccr::engine::tests::calculate_ead_rejects_trade_with_supervisory_delta_sign_mismatch` |
