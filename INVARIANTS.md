# finstack Workspace Invariants

Authoritative source for cross-crate invariants. Quant-audit PR 16 (P3 #31)
created this document to consolidate rules that were previously scattered
across docstrings, CLAUDE.md, and individual PR descriptions.

When adding code: read this file first. When modifying code: re-read to
confirm the invariant holds. When the truth on the ground diverges from
this document, either update the code **or** update this document —
never let them drift.

---

## 1. Decimal vs f64 for Money

The intent of the workspace is **Decimal for money, f64 for model
internals**. The current reality is mixed; this section documents which
crates are on which convention, so new code can follow the local rule
and migrations can be tracked per-crate.

| Crate | Convention | Status |
|-------|-----------|--------|
| `finstack-core::money` | `Decimal` internally, `f64` API surface | Intentional: ergonomic f64 surface, Decimal storage |
| `finstack-core` (everything else) | `f64` | By design — core math and types |
| `finstack-cashflows` | `Decimal` for `money × rate × yf` products | Enforced per-commit |
| `finstack-monte-carlo` | `f64` | By design — MC path simulation |
| `finstack-analytics` | `f64` | By design — statistical analytics |
| `finstack-margin` | `f64` at aggregation; `Money` at boundaries | Mixed; aggregation migration tracked as audit follow-up |
| `finstack-statements` | `f64` via `AmountOrScalar` | Audit finding C15 flags this; migration deferred |
| `finstack-statements-analytics` | `f64` | Inherits from statements |
| `finstack-portfolio` | `f64` via `Money::new(f64, …)` | Audit finding; Decimal migration tracked |
| `finstack-scenarios` | `f64` at scenario-apply | Fine |
| `finstack-valuations` | `f64` internals, `Money`/`Decimal` at boundaries | Fine |

**Rule for new code:**
* Money values that flow to accounting, settlement, regulatory capital,
  or margin-dispute outputs MUST be `Decimal` at the boundary. Internal
  arithmetic MAY be `f64` with Neumaier/Kahan summation and explicit
  rounding at the boundary back to Decimal.
* Everything else (greeks, vols, rates, correlations, returns, prices of
  derivatives) is `f64`.

---

## 2. Determinism

### 2.1 RNG

* All stochastic code MUST use a seeded RNG. No `thread_rng()` /
  `rand::random()` in library code.
* Monte Carlo paths use `finstack_monte_carlo::rng::PhiloxRng` with
  explicit `split(path_id)` for per-path streams. This guarantees
  bit-identical results across serial and rayon-parallel executions.
* Halton low-discrepancy sequences (e.g. multi-start calibration) are
  deterministic by construction; see
  `finstack_valuations::calibration::solver::multi_start` for the shared
  implementation (PR 4).

### 2.2 Hash-map iteration

* `std::collections::HashMap` uses SipHash with a **random** seed per
  process start. Its iteration order is deterministic *within* one
  process but varies run-to-run.
* Any map whose iteration order leaks to serialization, file output, or
  tests MUST be `IndexMap` (insertion-ordered) or `BTreeMap` (sorted).
* Internal lookups that never leak order MAY use `HashMap`. Examples:
  intern tables, memoization caches.

Known offender history:
* Pre-PR-16: `finstack_portfolio::metrics::aggregate_collected_metrics`
  used `HashMap<Arc<str>, Vec<f64>>` whose iteration order leaked into
  `PortfolioMetrics.aggregated`. Fixed in PR 16 (P3 #33).

### 2.3 Reduction

* Summations over `f64` MUST use Neumaier/Kahan compensation for any
  accumulator that sees > 1000 terms or large dynamic range. See
  `finstack_core::math::neumaier_sum`.
* Parallel reductions (rayon) MUST use an associative + commutative
  combiner — always true for `+` on `f64`, but Neumaier's
  compensation is NOT associative; use `OnlineStats::merge` (Welford)
  or pre-partition deterministically then compensate within each
  partition.

### 2.4 Floating-point comparisons

* NEVER compare `f64` with `==` or `!=`. Use
  `finstack_core::util::approx_eq` or a documented tolerance.
* When tolerance is a percentage of the value, spell it out: a `2%`
  tolerance on an FEP value is different from a `1e-6` absolute
  tolerance on a probability.

---

## 3. Sign conventions

These conventions are NOT yet enforced workspace-wide (audit finding
C17 — `statements` sign-convention refactor deferred). Document what
each interface expects:

| Context | Convention |
|---------|-----------|
| `finstack-portfolio` Dietz flow | **positive = contribution in** (capital added by client) |
| `finstack-core::cashflow::xirr` flow | **negative = contribution in** (PV-zero solver convention) |
| CDS option payoff | Call = payer protection; Put = receiver protection |
| SA-CCR replacement cost `margin_term` | NICA sign per BCBS 279 ¶135; unsigned today (audit M3 — follow-up) |
| CapEx on balance-sheet roll-forward | positive (additions to PPE) |
| CapEx on cash-flow statement | negative (outflow in CFI) |
| Dividends on retained-earnings roll-forward | positive (outflow magnitude) |

**These last three contradict each other on the same underlying fact.**
Audit finding C17 flags this; full remediation is PR 7b and tracked
separately.

---

## 4. Day-count and time conventions

* Year-fractions for DISCOUNTING use the curve's own day-count.
* Year-fractions for VOLATILITY use the instrument's own day-count
  (typically Act/365F for equities, Act/360 for rates).
* Multi-curve pricing MUST keep these two clocks separate. See audit
  finding P1 #19 (deferred to a dedicated PR): the current workspace
  uses a single clock and drifts by 2–15 bp on steep cross-currency
  curves.

---

## 5. Error handling

* Library code MUST NOT panic in the happy path. The workspace Clippy
  profile denies `unwrap_used`, `expect_used`, `panic`, `unreachable`,
  `indexing_slicing` in non-test code.
* Fall-through `match _ => { ... }` arms on error enums are denied by
  `match_wild_err_arm` — list each variant explicitly so a new variant
  forces a code review.
* New error variants on shared enums (`PricingError`, `ScenarioError`,
  `CalibrationError`) MUST include a message that cites:
  1. What the caller was trying to do.
  2. What went wrong.
  3. A pointer to the audit finding or roadmap entry that
     motivated the error path, if applicable — helps future readers
     understand WHY the error exists.

---

## 6. Test discipline

* TDD: failing test committed before or with the fix.
* Regression tests for audit findings MUST mention the finding ID
  (e.g. "C8", "P1 #18") in the test doc comment or the assertion
  message so grepping is fast.
* Golden-value tests (vs QuantLib, ISDA SIMM, GIPS, etc.) live in
  `<crate>/tests/` with explicit provenance in the test header.

---

## 7. Binding updates

* Any public API change on a core crate MUST update `finstack-py` and
  `finstack-wasm` in the same PR. "API-break-carry" is denied.
* Deprecation is preferred over removal when the API is still
  meaningful — e.g. PR 2 (quant-audit) removed `LewisPricer` outright
  because it was known-broken and had no salvageable contract, but
  most other changes should keep the old name working with a `#[deprecated]`
  annotation pointing at the replacement.

---

## 8. References

* Audit executive report: `docs/superpowers/plans/2026-04-19-quant-audit-remediation-roadmap.md`
* Per-PR detail in commit messages tagged `quant-audit C<N>` or `P<N> #<M>`.
* Hagan, P. S., & West, G. (2006) — monotone-convex interpolation
  (PR 0).
* Brigo & Mercurio (2006), Andersen (2008) — HW1F, QE-Heston (PRs 1, 5).
* ISDA SIMM Methodology v2.6 — SIMM aggregation (PR 7, deferred).
* BCBS-IOSCO *Margin Requirements* (2015/2020) — Schedule IM NGR
  (PR 14).
* CFA Institute GIPS Standards (2020) — TWRR / MWRR (PR 12).
* S&P / Moody's rating scorecard methodology — threshold boundaries
  (PR 15).

This list is not exhaustive; each commit message cites the specific
reference for that change.
