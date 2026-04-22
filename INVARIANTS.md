# finstack Workspace Invariants

Authoritative source for cross-crate invariants. This document
consolidates rules that would otherwise be scattered across docstrings
and individual commit descriptions.

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
| `finstack-margin` | `f64` at aggregation; `Money` at boundaries | Mixed; aggregation migration deferred |
| `finstack-statements` | `f64` via `AmountOrScalar` | Decimal migration deferred |
| `finstack-statements-analytics` | `f64` | Inherits from statements |
| `finstack-portfolio` | `f64` via `Money::new(f64, …)` | Decimal migration deferred |
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
  implementation.

### 2.2 Hash-map iteration

* `std::collections::HashMap` uses SipHash with a **random** seed per
  process start. Its iteration order is deterministic *within* one
  process but varies run-to-run.
* Any map whose iteration order leaks to serialization, file output, or
  tests MUST be `IndexMap` (insertion-ordered) or `BTreeMap` (sorted).
* Internal lookups that never leak order MAY use `HashMap`. Examples:
  intern tables, memoization caches.

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

These conventions are NOT yet enforced workspace-wide (statements sign-
convention refactor deferred). Document what each interface expects:

| Context | Convention |
|---------|-----------|
| `finstack-portfolio` Dietz flow | **positive = contribution in** (capital added by client) |
| `finstack-core::cashflow::xirr` flow | **negative = contribution in** (PV-zero solver convention) |
| CDS option payoff | Call = payer protection; Put = receiver protection |
| SA-CCR replacement cost `margin_term` | NICA sign per BCBS 279 ¶135; unsigned today (follow-up) |
| CapEx on balance-sheet roll-forward | positive (additions to PPE) |
| CapEx on cash-flow statement | negative (outflow in CFI) |
| Dividends on retained-earnings roll-forward | positive (outflow magnitude) |

**These last three contradict each other on the same underlying fact.**
Full remediation is tracked separately.

---

## 4. Day-count and time conventions

* Year-fractions for DISCOUNTING use the curve's own day-count.
* Year-fractions for VOLATILITY use the instrument's own day-count
  (typically Act/365F for equities, Act/360 for rates).
* Multi-curve pricing MUST keep these two clocks separate. Single-clock
  shortcuts drift by 2–15 bp on steep cross-currency curves; the
  two-clock plumbing lives in
  `finstack_valuations::instruments::common::two_clock` and is being
  migrated into pricer paths incrementally.

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
  3. Enough context for a future reader to understand WHY the error
     path exists.

---

## 6. Test discipline

* TDD: failing test committed before or with the fix.
* Regression tests MUST describe the failure mode they lock in (e.g.
  "κ out of `[0.001, 1]` bounds must `Err` rather than warn-and-
  succeed") in the test doc comment or assertion message so grepping
  is fast.
* Golden-value tests (vs QuantLib, ISDA SIMM, GIPS, etc.) live in
  `<crate>/tests/` with explicit provenance in the test header.

---

## 7. Binding updates

* Any public API change on a core crate MUST update `finstack-py` and
  `finstack-wasm` in the same commit. "API-break-carry" is denied.
* Deprecation is preferred over removal when the API is still
  meaningful. Outright removal is reserved for APIs that are known-
  broken and have no salvageable contract (e.g. the Lewis Fourier
  pricer, which was known-divergent off-ATM); most other changes
  should keep the old name working with a `#[deprecated]` annotation
  pointing at the replacement.

---

## 8. References

* Hagan, P. S., & West, G. (2006) — monotone-convex interpolation.
* Brigo & Mercurio (2006), Andersen (2008) — HW1F, QE-Heston.
* ISDA SIMM Methodology v2.6 — SIMM aggregation.
* BCBS-IOSCO *Margin Requirements* (2015/2020) — Schedule IM NGR.
* CFA Institute GIPS Standards (2020) — TWRR / MWRR.
* S&P / Moody's rating scorecard methodology — threshold boundaries.

This list is not exhaustive; each commit message cites the specific
reference for that change.
