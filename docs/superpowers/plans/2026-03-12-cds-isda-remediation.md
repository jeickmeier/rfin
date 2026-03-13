# CDS Family ISDA Compliance Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the CDS-family ISDA remediation PRD in safe, test-backed chunks, starting with single-name CDS lifecycle and valuation correctness.

**Architecture:** Build a shared lifecycle vocabulary first, then correct single-name CDS premium/protection-leg math, then normalize quote/build/calibration paths, and finally repair the index, tranche, option, schema, and release-fixture surfaces. Keep existing public APIs where possible, but prefer explicit validation and honest naming over preserving misleading behavior.

**Tech Stack:** Rust workspace (`finstack-valuations`, `finstack-core`), cargo tests, JSON schemas, markdown docs.

---

## Chunk 1: Single-Name CDS Lifecycle and Valuation Core

### Task 1: Add failing regression tests for AoD timing and `IsdaStandardModel` step invariance

**Files:**
- Modify: `finstack/valuations/tests/instruments/cds/test_cds_pricing.rs`
- Modify: `finstack/valuations/tests/instruments/cds/test_cds_integration_methods.rs`
- Reference: `finstack/valuations/src/instruments/credit_derivatives/cds/pricer.rs`

- [x] **Step 1: Write the failing AoD settlement-timing test**

Add a test that constructs a stressed single-name CDS with settlement lag and asserts premium-leg PV using AoD is higher when default accrual is discounted to default settlement timing instead of coupon payment timing. Name it `test_aod_uses_default_settlement_timing`.

- [x] **Step 2: Run the AoD test to verify it fails for the expected reason**

Run: `cargo test -p finstack-valuations test_aod_uses_default_settlement_timing -- --exact`

Expected: FAIL because the current implementation discounts AoD to coupon payment date.

- [x] **Step 3: Write the failing integration-invariance test**

Add a test that prices the same CDS under `IntegrationMethod::IsdaStandardModel` with materially different step controls and asserts the PVs are equal within tight tolerance. Name it `test_isda_standard_model_ignores_step_tuning`.

- [x] **Step 4: Run the invariance test to verify it fails**

Run: `cargo test -p finstack-valuations test_isda_standard_model_ignores_step_tuning -- --exact`

Expected: FAIL because the current implementation still depends on step controls.

- [x] **Step 5: Capture the failure mode in comments or test names only**

Do not add workaround logic in tests. Keep the tests expressing the desired market behavior.

### Task 2: Introduce explicit single-name CDS lifecycle helpers and fix AoD discount timing

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/pricer.rs`
- Test: `finstack/valuations/tests/instruments/cds/test_cds_pricing.rs`

- [ ] **Step 1: Add the next failing lifecycle test**

Add a focused test covering lifecycle date derivation for an on-the-run CDS trade, asserting that pricing can distinguish contract protection start from settlement timing. Keep it minimal and single-purpose.

- [ ] **Step 2: Run the lifecycle test to verify it fails**

Run: `cargo test -p finstack-valuations test_cds_lifecycle_dates_are_distinct -- --exact`

Expected: FAIL because lifecycle semantics are currently conflated.

- [ ] **Step 3: Implement minimal lifecycle helpers**

Add internal helpers for single-name CDS lifecycle semantics that can derive:
- protection start / step-in anchor
- cash settlement date
- default settlement date

Prefer helper methods and small structs over adding multiple loosely related boolean flags.

- [ ] **Step 4: Route AoD pricing through the lifecycle helpers**

Update `accrual_on_default_isda_midpoint()` and its callers so accrued premium is discounted to default settlement timing, not coupon payment date.

- [ ] **Step 5: Run the focused single-name CDS tests**

Run: `cargo test -p finstack-valuations test_aod_uses_default_settlement_timing test_cds_lifecycle_dates_are_distinct -- --exact`

Expected: PASS.

- [ ] **Step 6: Refactor only after green**

Collapse duplicated settlement-date calculations in `pricer.rs` into shared helpers without changing behavior beyond the tested fix.

### Task 3: Replace grid-dependent `IsdaStandardModel` integration with breakpoint-based interval integration

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/pricer.rs`
- Reference: `finstack/core/src/market_data/term_structures/hazard_curve.rs`
- Test: `finstack/valuations/tests/instruments/cds/test_cds_integration_methods.rs`

- [ ] **Step 1: Add one exact flat-curve test if missing**

Add a flat hazard / flat discount case where the `IsdaStandardModel` path should match a manually stable expected value under large step-size variation.

- [ ] **Step 2: Run the new flat-curve test to verify it fails**

Run: `cargo test -p finstack-valuations test_isda_standard_model_flat_curve_exactness -- --exact`

Expected: FAIL or show step-dependent drift.

- [ ] **Step 3: Implement interval-boundary collection**

Build a sorted, deduplicated grid from:
- protection interval start/end
- hazard knot times
- relevant mapped discount/settlement boundaries

- [ ] **Step 4: Implement analytical interval integration**

For each interval, integrate the protection leg using locally constant hazard and discount behavior. Remove use of equal-step tuning for the `IsdaStandardModel` path.

- [ ] **Step 5: Run the focused integration tests**

Run: `cargo test -p finstack-valuations test_isda_standard_model_ignores_step_tuning test_isda_standard_model_flat_curve_exactness -- --exact`

Expected: PASS.

- [ ] **Step 6: Run the broader CDS integration suite**

Run: `cargo test -p finstack-valuations cds::test_cds_integration_methods`

Expected: PASS without tolerance loosening.

### Task 4: Update single-name docs to match the implemented behavior

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/README.md`
- Modify: `docs/cds-isda-remediation-prd.md`

- [x] **Step 1: Remove or rewrite any inaccurate “ISDA standard” wording**

Document the actual AoD timing and exact-interval behavior that now exists.

- [ ] **Step 2: Run markdown checks only if touched docs trigger formatting issues**

Run: `make pre-commit-run`

Expected: PASS or report concrete failures to fix before moving on.

---

## Chunk 2: Standard Quote Normalization and Canonical Hazard Calibration

### Task 5: Preserve convention/doc-clause identity and normalize standard CDS quotes

**Files:**
- Modify: `finstack/valuations/src/market/build/cds.rs`
- Modify: `finstack/valuations/src/market/build/prepared.rs`
- Modify: `finstack/valuations/src/market/quotes/cds.rs`
- Test: `finstack/valuations/tests/market/build/credit.rs`
- Test: `finstack/valuations/tests/calibration/repricing.rs`

- [x] **Step 1: Write failing tests for convention/doc-clause roundtrip and standard-coupon upfront quote handling**
- [x] **Step 2: Run those tests to verify they fail**
- [x] **Step 3: Implement canonical quote normalization for par spread vs standard-coupon upfront**
- [x] **Step 4: Preserve convention and doc-clause metadata in built instruments**
- [x] **Step 5: Reprice normalized quotes through the existing calibration path and verify tests pass**

### Task 6: Remove or delegate duplicate bootstrap logic

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/pricer.rs`
- Modify: `finstack/valuations/src/calibration/targets/hazard.rs`
- Test: `finstack/valuations/tests/calibration/hazard_curve.rs`

- [x] **Step 1: Write a failing regression test for day-count / pillar consistency in CDS bootstrap**
- [x] **Step 2: Run it to verify failure**
- [x] **Step 3: Delegate or remove `CDSBootstrapper::bootstrap_hazard_curve()` in favor of the canonical target/solver path**
- [x] **Step 4: Re-run targeted calibration tests until green**

---

## Chunk 3: CDS Index and Tranche Contract Anchoring

### Task 7: Anchor CDS index lifecycle and default calendar behavior

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_index/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_index/pricer.rs`
- Test: `finstack/valuations/tests/instruments/cds_index/market_conventions.rs`
- Test: `finstack/valuations/tests/instruments/cds_index/pricing_single_curve.rs`

- [x] **Step 1: Write failing tests for calendar defaults and clean/dirty settlement semantics**
- [x] **Step 2: Run them to verify failure**
- [x] **Step 3: Implement explicit standard-index lifecycle defaults**
- [x] **Step 4: Re-run focused CDS index tests**

### Task 8: Contract-anchor tranche schedules and fix accrued-premium behavior

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/pricer.rs`
- Test: `finstack/valuations/tests/instruments/cds_tranche/market_standards_tests.rs`
- Test: `finstack/valuations/tests/instruments/cds_tranche/pricing_tests.rs`

- [x] **Step 1: Write failing seasoned-trade tranche tests around coupon-date accrual**
- [x] **Step 2: Run them to verify failure**
- [x] **Step 3: Require or derive contractual effective dates for standard tranches**
- [x] **Step 4: Recompute accrued premium from the actual historical schedule**
- [x] **Step 5: Re-run tranche pricing and market-standard tests**

### Task 9: Fix schema examples for CDS, CDS index, and tranche

**Files:**
- Modify: `finstack/valuations/schemas/instruments/1/credit_derivatives/credit_default_swap.schema.json`
- Modify: `finstack/valuations/schemas/instruments/1/credit_derivatives/cds_index.schema.json`
- Modify: `finstack/valuations/schemas/instruments/1/credit_derivatives/cds_tranche.schema.json`

- [x] **Step 1: Update examples to use supported market-default conventions**
- [x] **Step 2: Fix tranche percent-point example units**
- [x] **Step 3: Run schema checks via `make pre-commit-run`**

---

## Chunk 4: CDS Option Product Surface, Release Fixtures, and Final Compliance Gate

### Task 10: Narrow the CDS option product surface to supported behavior

**Files:**
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_option/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_option/pricer.rs`
- Modify: `finstack/valuations/schemas/instruments/1/credit_derivatives/cds_option.schema.json`
- Test: `finstack/valuations/tests/instruments/cds_option/test_types.rs`
- Test: `finstack/valuations/tests/instruments/cds_option/test_pricing.rs`
- Test: `finstack/valuations/tests/instruments/cds_option/test_index_options.rs`

- [x] **Step 1: Write failing validation tests for unsupported settlement/exercise combinations**
- [x] **Step 2: Run them to verify failure**
- [x] **Step 3: Implement fail-fast validation for unsupported configurations**
- [x] **Step 4: Update schema/docs so the public surface matches the implementation**
- [x] **Step 5: Re-run focused CDS option tests**

### Task 11: Add release-gate fixtures and tighten parity coverage

**Files:**
- Modify: `finstack/valuations/tests/quantlib_parity/test_cds_parity.rs`
- Create or modify: `finstack/valuations/tests/instruments/cds/test_cds_market_validation.rs`
- Create or modify: `finstack/valuations/tests/instruments/cds_index/market_conventions.rs`
- Create or modify: `finstack/valuations/tests/instruments/cds_tranche/market_standards_tests.rs`

- [x] **Step 1: Add frozen lifecycle fixtures for NA, EU, and Asia CDS**
- [x] **Step 2: Add clean-vs-dirty settlement and step-in-date assertions**
- [x] **Step 3: Run targeted parity fixtures and note exact tolerances in test names or comments**
- [x] **Step 4: Run the broad CDS-family test sweep**

Run:
- `cargo test -p finstack-valuations cds`
- `cargo test -p finstack-valuations cds_index`
- `cargo test -p finstack-valuations cds_tranche`
- `cargo test -p finstack-valuations cds_option`

Expected: PASS.

---

## Execution Notes

- Start with Chunk 1 only. It is the foundation for the rest of the PRD and touches the highest-risk valuation logic.
- Do not commit unless the user explicitly asks for a commit.
- After each task, run the focused tests before touching the next task.
- Before any completion claim, run fresh verification commands and report the actual outputs.
