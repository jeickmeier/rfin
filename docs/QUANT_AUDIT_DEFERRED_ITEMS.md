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
| **Merged to master** | 34 | C1, C2, C3, C4, C5, C6, C7, C8, C9 (partial), C17, C18, C19, C20, P1 #15, P1 #16, P1 #17, P1 #18, P1 #20, P1 #21, P1 #22, P1 #23 (partial), P2 #25, P2 #26, P2 #27, P2 #28, P2 #29, P2 #30, P3 #31, P3 #32, P3 #33, P3 #34, P3 #35, **N2** |
| Deferred | 3 | C9 (co-terminal repricer), C10–C14, C15 (full Decimal migration), P1 #19, P2 #24 |
| Re-classified | 1 | **N1** (see §New findings) |

34/35 + 1 new audit findings materially closed. Remaining 3 deferred
with specific reasons below.

### Recently closed — follow-up session 3 (2026-04-21 cont.)

The statements refactor (C15–C20) and its blocked dependency P1 #17
landed in this sweep:

- **C17 SignConventionPolicy** — new `SignConventionPolicy` enum in
  `finstack_statements::checks::types` with `MagnitudePositive` /
  `InflowPositive` variants and a `validate()` method that emits
  `Severity::Warning` findings for convention violations. Wired into
  `CapexReconciliation`, `DepreciationReconciliation`,
  `DividendReconciliation`, and `RetainedEarningsReconciliation` so a
  misconfigured sign now surfaces as a typed warning rather than a
  silent reconciliation mismatch.
- **C18 TTM guard** — verified closed in prior work at
  `formula_aggregates.rs:303`: TTM returns NaN when `values.len() <
  window` or any value is non-finite. Test expectations in
  `test_ttm_function`, `test_ttm_with_nan`, and
  `test_ttm_function_supports_expression_arguments` brought into
  alignment with the C18-compliant behavior.
- **C19 as_of PIT filter** — verified closed in prior work at
  `engine.rs:228-231`. Future-period actual values are hidden when
  `as_of` is set and `period.start > as_of_date`, forcing the
  evaluator through the forecast / formula precedence path.
- **C20 average-balance interest reconciliation** — fixed
  `InterestExpenseReconciliation::execute` to use `(B_{t−1} + B_t) / 2`
  instead of the EOP balance when computing implied interest against
  debt balance × rate. Regression test
  `interest_implied_rate_uses_average_balance_audit_c20` anchors the
  formula on a rising-balance scenario (800 → 1200, rate 5 %, interest
  50 = avg × rate) that would fail under the old EOP convention.
- **C15 Decimal-at-boundary helper** — full migration of
  `EvaluationContext::current_values` to `Vec<Option<Decimal>>`
  remains deferred (100+ call-site ripple, semver-breaking). In the
  meantime, `EvaluationContext::get_value_decimal(node_id)` converts
  the internally-stored `f64` to `rust_decimal::Decimal` at the
  caller's boundary, rejecting non-finite values explicitly rather
  than propagating them as `Decimal::ZERO`. Downstream accounting /
  settlement code paths that need the workspace money invariants
  (INVARIANTS.md §1) can opt in today without forcing every consumer
  to migrate.
- **P1 #17 Adjusted Net Debt bridge** — new
  `finstack_statements_analytics::analysis::credit::adjusted_net_debt`
  module with `AdjustedNetDebtSpec` (+ `AdjustedNetDebtSpecBuilder`)
  implementing `Debt − Cash − MarketableSecurities + Leases +
  Pension + OtherAdditions − OtherSubtractions`. `compute()` produces
  a single-period value and `compute_series(&results)` produces an
  `IndexMap<PeriodId, f64>` preserving the evaluator's period
  ordering. This unblocks rating-agency-style leverage reporting via
  the existing covenant engine.
- **Statements build unblocked** — prior uncommitted edit at
  `precedence.rs:33` was refactored away from Rust 2024 let-chain
  syntax; `finstack-statements` now builds clean on edition 2021.

### Recently closed — follow-up session 2 (2026-04-21 cont.)

Seven additional items landed in a third follow-up sweep:

- **P3 #34 Jacobi eigensolver replaced** — exposed
  `finstack_core::math::linalg::symmetric_eigen` as a workspace helper,
  routed `valuations::correlation::nearest_correlation::project_psd`
  through it, and retired the hand-rolled Jacobi sweep (O(n⁵) worst
  case for `n > 40`). Regression test at `n = 60` verifies the
  projection still produces a valid correlation matrix in the regime
  where the old solver was slow.
- **P3 #32 `FactorBumpUnit` enum** — added
  `finstack_core::factor_model::FactorBumpUnit { Absolute, BasisPoint,
  Percent, Fraction, Multiplier }` alongside
  `BumpSizeConfig::bump_size_with_unit_for_factor`.
  `mapping_to_market_bumps` now takes the unit and validates against
  the mapping's declared `BumpUnits` (or converts for `EquitySpot` /
  `FxRate`), catching a rates-bp-into-percent-mapping at bump
  construction instead of as a silent 100× scaling error.
- **P3 #35 version-string consolidation** — created
  `finstack_core::versions` with the five canonical calibration model
  strings (Hull-White 1F, Multi-curve OIS, ISDA Standard Model v1.8.2,
  Student-t Copula v1.0, SVI v1.1). All five call sites now consume
  the constants so a future revision bump touches one file.
- **P2 #29 Vanna-Volga FX barrier wired** —
  `FxBarrierOptionVannaVolgaPricer` now implements the `Pricer` trait,
  is registered under `ModelKey::FxBarrierVannaVolga = 27` in
  `pricer/fx.rs`, and default-constructs with a symmetric-smile
  fallback (equivalent to BS analytical) until callers bind explicit
  market quotes via `with_quotes(...)`. Smoke test verifies the VV
  and BS pricers agree exactly under the symmetric-fallback smile.
- **P2 #27 Brinson-Fachler attribution** — new
  `finstack_portfolio::brinson` module implements the classical
  three-way decomposition (Allocation / Selection / Interaction) with
  Carino (1999) linking for multi-period compounding. Tests verify
  the definitional invariants: single-period A+S+I = active return
  exactly; multi-period Carino-linked A+S+I = geometric active return
  exactly; inputs with malformed weights rejected.
- **P1 #16 multi-sector generalization (K > 2)** —
  `MultiFactorCopula::with_k_sectors(num_sectors, β_G, β_S)` now
  supports up to 4 sector factors (+ 1 global = 5 total), replacing
  the previous 2-factor hard cap. `conditional_default_prob_with_sector`
  routes `sector_idx ∈ 1..=K` to the `sector_idx`-th slot of the
  factor realization. `integrate_fn` uses a recursive nested
  Gauss-Hermite quadrature with per-dimension `1/√π` normalization
  matching `GaussHermiteQuadrature::integrate` — exact through any
  `K ≤ MAX_SECTORS`. New tests verify sector routing and integration
  of unconditional PD for each sector.
- **P2 #25 commodity matrix** (carried over from the first follow-up
  sweep; now fully complete — registry-load PSD validation covers
  risk-class, IR tenor, CQ inter-bucket, and commodity 17×17).

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
  - τ·w(k, T₂)` which preserves calendar-spread monotonicity by
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
  - one sector) and a true `K`-sector copula with
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

#### P2 #27 — Brinson-Fachler attribution in portfolio — ✅ CLOSED (2026-04-21 follow-up 2)

- **Original roadmap target:** PR 15 first sub-item.
- **Closed in:** 2026-04-21 follow-up 2. New
  `finstack_portfolio::brinson` module: `brinson_fachler(sectors)`
  produces the three-way decomposition per period, `carino_link(periods)`
  applies the Carino (1999) smoothing so arithmetic effects reconstruct
  the geometrically compounded active return. Currency attribution and
  a full multi-period portfolio fixture remain as a follow-up;
  industry-standard implementations layer currency on top of sector
  Brinson, which is straightforward given the scaffolding now in place.

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

#### P2 #29 — Vanna-Volga FX + digital replication — ✅ CLOSED (2026-04-21 follow-up 2)

- **Original roadmap target:** PR 15 third sub-item.
- **Closed in:** 2026-04-21 follow-up 2.
  `FxBarrierOptionVannaVolgaPricer` now implements the `Pricer` trait,
  is registered at `ModelKey::FxBarrierVannaVolga = 27` in
  `pricer/fx.rs`, and default-constructs with a symmetric-smile
  fallback (equivalent to BS) so registering is safe for instruments
  that haven't yet had market smile quotes wired in. Production FX
  books must call `with_quotes(...)` at pricer construction time with
  real 25Δ market data. FX digitals with VV smile remain a separate
  follow-up; the barrier path now demonstrates the wiring pattern.

### Tier P3 — hygiene ✅ FULLY CLOSED (2026-04-21 follow-up 2)

All P3 items (P3 #31, #32, #33, #34, #35) are merged. See the
"Recently closed" sections and the cross-reference index for the
specific commits and implementation locations. The hygiene tier is
retired; future refactor-only items should be tracked in the general
backlog rather than tagged P3.

---

## Deferred totals by tier

| Tier | Deferred findings | Estimated effort |
|------|-------------------|------------------|
| P0 (ship-blockers) | C9 part 3, C10–C14, C15 (full Decimal migration) | ~3 weeks |
| P1 | #19 | ~2 weeks |
| P2 | #24 | ~2 days |
| P3 | — | — |

**Total remaining effort:** approximately 5 engineer-weeks across 3
findings (was 9 weeks / 19 findings before the 2026-04-21 follow-up
sessions). The remaining items are:

- **C10–C14 SIMM rewrite** — blocked on ISDA v2.6 public test fixtures
  being committed to the repo.
- **C15 full Decimal migration** — semver-breaking refactor of
  `EvaluationContext::current_values` from `Vec<Option<f64>>` to
  `Vec<Option<Decimal>>`, rippling through 100+ formula / check call
  sites. The [`get_value_decimal`] boundary helper partially mitigates;
  full migration deferred to a future major version.
- **C9 part 3 LMM independent repricer** — ~2 days; needs
  Longstaff-Schwartz or nested Monte Carlo scaffolding.
- **P1 #19 r_eff two-clock plumbing** — ~2 weeks, 12+ pricer migration
  with per-pricer QuantLib golden values.
- **P2 #24 FRTB parameter JSON registry** — ~2 days; requires threading
  `FrtbParams` through the 5 charge-calc files.

The C18, C19, C20, C17, and P1 #17 items that were originally grouped
under the "C15–C20 statements refactor" P0 block are all now closed
independently — C15's full Decimal migration is the lone remaining
piece and is its own separate major-version item.

---

## Recommendations for next sprints

### Sprint priorities

1. **C15–C20 statements refactor** — highest downstream impact
   (blocks PR 9 = P1 #17). Start with C18 (TTM `min_periods`) and C19
   (PIT filter) as independent PRs before tackling the sign-convention
   refactor. This remains the single largest open item on the board.
2. **C10–C14 SIMM rewrite** — regulatory-critical. Sequence:
   (a) commit ISDA v2.6 fixtures, (b) write parity runner, (c) fix
   aggregation head. Do not skip step (a). This is the only remaining
   compliance gating item.
3. **P1 #19 r_eff plumbing** — measurable pricing impact on
   cross-currency books; one-pricer-at-a-time migration plan. Can be
   parallelized with the SIMM rewrite since they touch disjoint code.
4. **P2 #24 FRTB parameter JSON registry** — 2-day scaffold pending;
   adds a `FrtbParams` struct bundling the currently-hardcoded GIRR /
   CSR / equity / commodity / FX constants and wires JSON overlay
   loading. Gates side-by-side regression testing of BCBS d457 vs
   d554 FRTB revisions.

### Process notes

- Each remaining PR should follow the same discipline as the 13
  landed: failing regression test committed with the fix, citation of
  the audit finding ID in both the test and the commit message, clean
  clippy under `-D warnings`, and a scope-note in the commit message
  listing what was explicitly deferred further.
- The 13 merged PR branches (`feat/quant-audit-pr0-hagan-west`
  through `-pr16-hygiene`) are retained on the local repository for
  cherry-picking reference — they can be pruned with
  `git branch -d feat/quant-audit-*` once no follow-ups need them.
- `INVARIANTS.md` at the repo root now documents the cross-crate
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
| P2 #27 | merged (2026-04-21 follow-up 2) | `finstack_portfolio::brinson` module — three-way BF decomposition + Carino linking; tests assert A+S+I = active return exactly (single period) and geometric active return exactly (linked). |
| P2 #28 | merged (2026-04-21 follow-up) | Gatheral total-variance interpolation in `calibration/targets/svi.rs::interpolate_svi_vol`; post-build `SurfaceValidator::{validate_calendar_spread, validate_butterfly_spread}` wired with `lenient_arbitrage = true`. |
| P2 #29 | merged (2026-04-21 follow-up 2) | `FxBarrierOptionVannaVolgaPricer` now implements `Pricer`, registered under `ModelKey::FxBarrierVannaVolga = 27` in `pricer/fx.rs`. Symmetric-smile fallback for default-constructed pricers; `with_quotes(...)` for explicit 25Δ vols. |
| P2 #30 | merged | `698ae9ef4` PR 15 |
| P3 #31 | merged | `31f25a621` PR 16 |
| P3 #32 | deferred | §FactorBumpUnit |
| P3 #33 | merged | `31f25a621` PR 16 |
| P3 #32 | merged (2026-04-21 follow-up 2) | `FactorBumpUnit` enum in `finstack_core::factor_model` + `BumpSizeConfig::bump_size_with_unit_for_factor`; `mapping_to_market_bumps` validates against mapping `BumpUnits` or converts for `EquitySpot`/`FxRate`. |
| P3 #34 | merged (2026-04-21 follow-up 2) | `finstack_core::math::linalg::symmetric_eigen` exposed as workspace helper; `nearest_correlation::project_psd` delegates to it, retiring the O(n⁵) Jacobi sweep. |
| P3 #35 | merged (2026-04-21 follow-up 2) | `finstack_core::versions` module consolidates the five canonical model-version strings (Hull-White 1F, Multi-curve OIS, ISDA Standard Model, Student-t Copula, SVI). All call sites migrated. |
| N1 (new) | re-classified into C10–C14 | §New findings — FX aggregation form |
| N2 (new) | merged (2026-04-21 session) | `SaCcrTrade::validate` in `margin/src/regulatory/sa_ccr/types.rs`; tests in `sa_ccr::types::validate_tests::*` and `sa_ccr::engine::tests::calculate_ead_rejects_trade_with_supervisory_delta_sign_mismatch` |
