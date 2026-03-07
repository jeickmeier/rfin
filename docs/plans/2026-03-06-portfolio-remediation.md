# Portfolio Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the highest-impact portfolio economics and optimizer correctness issues identified in the quant review, while adding regression tests that prove the repaired behavior.

**Architecture:** The work is split into two phases. Phase 1 repairs portfolio valuation/reporting correctness so position sizing, degraded-risk behavior, attribution closure, and portfolio structure are economically sound. Phase 2 hardens the optimizer so its public API matches the semantics it actually enforces, with unsupported behaviors either implemented or rejected explicitly.

**Tech Stack:** Rust workspace, `finstack-portfolio`, cargo tests, portfolio integration tests, optimization LP backend (`good_lp`)

---

## Task 1: Metric Scaling And Risk Visibility

**Files:**
- Modify: `finstack/portfolio/src/valuation.rs`
- Modify: `finstack/portfolio/src/metrics.rs`
- Modify: `finstack/portfolio/src/results.rs`
- Test: `finstack/portfolio/tests/aggregation_metrics.rs`
- Test: `finstack/portfolio/tests/valuation_fallback.rs`

**Step 1: Write the failing tests**

Add tests proving:
- summable metrics scale with `Position.quantity`
- short positions flip metric sign
- non-strict valuation surfaces degraded-risk information instead of silently looking complete

**Step 2: Run the targeted tests to verify they fail**

Run:
- `cargo test -p finstack-portfolio aggregation_metrics`
- `cargo test -p finstack-portfolio valuation_fallback`

Expected: FAIL because metrics are currently aggregated from unscaled `ValuationResult::measures` and fallback valuations do not expose degradation metadata.

**Step 3: Write minimal implementation**

Implement:
- position-level metric scaling before portfolio aggregation
- an explicit degraded-risk summary on valuation/results types
- propagation of fallback/missing-metric state from valuation into reporting

**Step 4: Run the targeted tests to verify they pass**

Run:
- `cargo test -p finstack-portfolio aggregation_metrics`
- `cargo test -p finstack-portfolio valuation_fallback`

Expected: PASS

## Task 2: Attribution Closure And Portfolio Integrity

**Files:**
- Modify: `finstack/portfolio/src/attribution.rs`
- Modify: `finstack/portfolio/src/builder.rs`
- Modify: `finstack/portfolio/src/portfolio.rs`
- Modify: `finstack/portfolio/src/position.rs`
- Test: `finstack/portfolio/tests/attribution_golden.rs`
- Test: `finstack/portfolio/tests/book_hierarchy_test.rs`
- Test: `finstack/portfolio/tests/serialization.rs`

**Step 1: Write the failing tests**

Add tests proving:
- attribution closes exactly under the selected FX translation convention
- book parent/child links are correct regardless of insertion order
- reassigning a position between books does not leave stale membership
- `from_spec()` preserves `book_id`, `tags`, and `meta`

**Step 2: Run the targeted tests to verify they fail**

Run:
- `cargo test -p finstack-portfolio attribution_golden`
- `cargo test -p finstack-portfolio book_hierarchy_test`
- `cargo test -p finstack-portfolio serialization`

Expected: FAIL because the current implementation does not enforce these invariants.

**Step 3: Write minimal implementation**

Implement:
- one consistent portfolio attribution identity
- builder/portfolio validation for hierarchy consistency and unique membership
- full `PositionSpec` round-trip restoration of portfolio metadata

**Step 4: Run the targeted tests to verify they pass**

Run:
- `cargo test -p finstack-portfolio attribution_golden`
- `cargo test -p finstack-portfolio book_hierarchy_test`
- `cargo test -p finstack-portfolio serialization`

Expected: PASS

## Task 3: Optimizer Semantics Hardening

**Files:**
- Modify: `finstack/portfolio/src/optimization/decision.rs`
- Modify: `finstack/portfolio/src/optimization/lp_solver.rs`
- Modify: `finstack/portfolio/src/optimization/types.rs`
- Modify: `finstack/portfolio/src/optimization/universe.rs`
- Modify: `finstack/portfolio/src/optimization/result.rs`
- Test: `finstack/portfolio/tests/optimization_basic.rs`
- Test: `finstack/portfolio/tests/test_optimization_fixes.rs`

**Step 1: Write the failing tests**

Add tests proving:
- excluded positions remain part of full-portfolio denominators and constraints as fixed sleeves, or are rejected explicitly
- `MaxPositionDelta` is enforced or rejected with a clear error
- `MissingMetricPolicy::Exclude` freezes affected positions instead of treating missing metrics as zero
- unsupported `PvNative` / short-candidate semantics are not silently accepted
- optimized weights reconstruct quantities consistently for the active weighting scheme

**Step 2: Run the targeted tests to verify they fail**

Run:
- `cargo test -p finstack-portfolio optimization_basic`
- `cargo test -p finstack-portfolio test_optimization_fixes`

Expected: FAIL because current optimizer behavior does not match the advertised semantics.

**Step 3: Write minimal implementation**

Implement:
- a single, coherent denominator model per weighting scheme
- explicit handling for excluded sleeves
- enforcement or rejection for currently misleading API features
- status/reporting updates where solver semantics are intentionally limited

**Step 4: Run the targeted tests to verify they pass**

Run:
- `cargo test -p finstack-portfolio optimization_basic`
- `cargo test -p finstack-portfolio test_optimization_fixes`

Expected: PASS

## Task 4: End-To-End Verification

**Files:**
- Re-run only

**Step 1: Run focused checks**

Run:
- `cargo test -p finstack-portfolio`

**Step 2: Run lint checks for touched code**

Run:
- `cargo clippy -p finstack-portfolio --all-features --tests -- -D warnings`

**Step 3: Fix any regressions**

Apply only the minimal changes needed to restore green tests and clean lint output.

**Step 4: Final verification**

Re-run:
- `cargo test -p finstack-portfolio`
- `cargo clippy -p finstack-portfolio --all-features --tests -- -D warnings`
