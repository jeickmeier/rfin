# Statements Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the highest-impact correctness defects in `finstack/statements` so statement formulas, capital-structure outputs, valuation analysis, and audit/normalization features behave according to mathematically correct practitioner semantics.

**Architecture:** Keep the existing module structure, but remediate behavior in four tracks: evaluator semantics, capital-structure cashflow correctness, analysis/DCF/scenario correctness, and audit/normalization semantics. Each behavior change must start with a failing regression test, followed by the minimal code change to make it pass, then focused verification before moving to the next bug.

**Tech Stack:** Rust, Cargo, `finstack-statements`, `finstack-core`, `finstack-valuations`, inline unit tests, crate integration tests under `finstack/statements/tests`

---

## Task 1: Establish Baseline and Test Targets

**Files:**
- Modify: `finstack/statements/tests/functions/custom_functions_tests.rs`
- Modify: `finstack/statements/tests/integration/waterfall_tests.rs`
- Modify: `finstack/statements/tests/analysis_corporate.rs`
- Modify: `finstack/statements/tests/analysis_scenario_set.rs`
- Modify: `finstack/statements/src/extensions/corkscrew.rs`
- Modify: `finstack/statements/src/adjustments/engine.rs`

**Step 1: Write the failing tests**

Add targeted regressions for:
- `ttm(revenue - cogs)` using a known quarterly gross-profit series
- multi-instrument ECF sweep conservation
- receive-fixed swap sign preservation in capital-structure aggregation
- DCF excluding historical periods and using valuation-date-consistent net debt
- scenario override preserving actual history
- corkscrew absolute tolerance semantics
- negative EBITDA cap behavior

**Step 2: Run tests to verify they fail**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test custom_functions_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test waterfall_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_corporate
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_scenario_set
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements corkscrew
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements adjustments::engine
```

Expected: failures that directly point to current incorrect behavior.

**Step 3: Do not change production code yet**

Confirm the failures are real behavior failures rather than malformed tests.

**Step 4: Commit checkpoint**

Do not commit unless explicitly requested by the user.

## Task 2: Fix Expression-Aware Historical Evaluation

**Files:**
- Modify: `finstack/statements/src/evaluator/formula.rs`
- Modify: `finstack/statements/tests/functions/custom_functions_tests.rs`
- Modify: `finstack/statements/tests/evaluator_tests.rs`

**Step 1: Write the failing test**

Use the regression from Task 1 for:
- `ttm(revenue - cogs)`
- `rolling_mean(revenue - cogs, 4)`
- `cumsum(revenue - cogs)` if needed to pin broader semantics

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test custom_functions_tests
```

Expected: current implementation evaluates complex expressions only in the current period.

**Step 3: Write minimal implementation**

In `src/evaluator/formula.rs`:
- add helper logic to evaluate an expression over historical contexts period-by-period
- use it in rolling/statistical/cumulative/history-based functions where non-column expressions are currently degraded
- keep existing column fast paths where appropriate

**Step 4: Run test to verify it passes**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test custom_functions_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements evaluator_tests
```

Expected: targeted regressions pass with no unrelated failures.

## Task 3: Fix `quantile()`, `coalesce()`, and Integer Argument Validation

**Files:**
- Modify: `finstack/statements/src/evaluator/formula.rs`
- Modify: `finstack/statements/tests/functions/custom_functions_tests.rs`
- Modify: `finstack/statements/tests/functions/nan_handling_tests.rs`

**Step 1: Write the failing tests**

Add tests for:
- `quantile()` on a four-point sample with an unambiguous median
- `coalesce(0, 5)` returning `0`
- `lag(x, 1.5)` / rolling-window with non-integer argument failing explicitly

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test custom_functions_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test nan_handling_tests
```

**Step 3: Write minimal implementation**

In `src/evaluator/formula.rs`:
- stop double-adding current values in `quantile()`
- treat zero as a valid `coalesce()` value
- reject non-finite or non-integer lag/window arguments instead of truncating them

**Step 4: Run test to verify it passes**

Run the same targeted tests and confirm green.

## Task 4: Tighten Monetary Semantics

**Files:**
- Modify: `finstack/statements/src/evaluator/engine.rs`
- Modify: `finstack/statements/src/builder/model_builder.rs`
- Modify: `finstack/statements/src/types/value.rs`
- Modify: `finstack/statements/tests/integration/money_integration_tests.rs`
- Modify: `finstack/statements/tests/model_builder.rs`

**Step 1: Write the failing test**

Add tests that:
- reject or surface invalid mixed-currency formula behavior
- preserve monetary typing for safe same-currency derived nodes
- reject inconsistent currencies within `value_money()`

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test money_integration_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test model_builder
```

**Step 3: Write minimal implementation**

Apply the smallest safe type-propagation and validation changes needed to make the tests pass.

**Step 4: Run test to verify it passes**

Run the same tests again.

## Task 5: Fix Waterfall Sweep Conservation

**Files:**
- Modify: `finstack/statements/src/capital_structure/waterfall.rs`
- Modify: `finstack/statements/tests/integration/waterfall_tests.rs`
- Modify: `finstack/statements/src/capital_structure/waterfall.rs`

**Step 1: Write the failing test**

Add a multi-instrument case where one ECF amount is currently over-applied.

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test waterfall_tests
```

**Step 3: Write minimal implementation**

Make sweep allocation consume a single remaining sweep pool across instruments instead of reusing the original amount for each one.

**Step 4: Run test to verify it passes**

Run the same test target again.

## Task 6: Restore Signed Capital-Structure Cashflow Economics

**Files:**
- Modify: `finstack/statements/src/capital_structure/integration.rs`
- Modify: `finstack/statements/tests/capital_structure_integration.rs`
- Modify: `finstack/statements/tests/audit_accrual.rs`

**Step 1: Write the failing test**

Add a receive-fixed or offsetting-flow case showing that absolute-value classification overstates interest expense.

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test capital_structure_integration
```

**Step 3: Write minimal implementation**

Preserve economic sign where appropriate and classify flows into statement buckets without flattening everything to absolute values.

**Step 4: Run test to verify it passes**

Run the same target and any affected accrual tests.

## Task 7: Fix DCF Valuation-Date Semantics

**Files:**
- Modify: `finstack/statements/src/analysis/corporate.rs`
- Modify: `finstack/statements/tests/analysis_corporate.rs`
- Modify: `finstack/statements/tests/analysis_orchestrator.rs`

**Step 1: Write the failing test**

Add regressions for:
- historical periods excluded from explicit DCF cashflows
- valuation-date-consistent net debt
- market-aware evaluation path actually using market-aware statement evaluation

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_corporate
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_orchestrator
```

**Step 3: Write minimal implementation**

Update DCF flow extraction and net-debt derivation to align with valuation semantics and wire market-aware evaluation correctly.

**Step 4: Run test to verify it passes**

Run the same targets again.

## Task 8: Fix Scenario Override Semantics

**Files:**
- Modify: `finstack/statements/src/analysis/scenario_set.rs`
- Modify: `finstack/statements/tests/analysis_scenario_set.rs`

**Step 1: Write the failing test**

Pin that actual periods remain intact and overrides apply only to intended forecast periods.

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_scenario_set
```

**Step 3: Write minimal implementation**

Change override application to preserve actual history and apply scenario semantics only where they belong.

**Step 4: Run test to verify it passes**

Run the same target again.

## Task 9: Fix Corkscrew Tolerance and Adjustment Cap Semantics

**Files:**
- Modify: `finstack/statements/src/extensions/corkscrew.rs`
- Modify: `finstack/statements/src/adjustments/engine.rs`
- Modify: `finstack/statements/tests/extensions/extensions_full_execution_tests.rs`
- Modify: `finstack/statements/src/adjustments/engine.rs`

**Step 1: Write the failing test**

Add tests that:
- prove `tolerance = 0.01` is treated as an absolute threshold
- prove negative EBITDA does not create positive cap headroom by default

**Step 2: Run test to verify it fails**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements extensions
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements adjustments::engine
```

**Step 3: Write minimal implementation**

Align each module with its documented semantics.

**Step 4: Run test to verify it passes**

Run the same focused targets again.

## Task 10: Full Verification and Cleanup

**Files:**
- Modify: any touched files from previous tasks

**Step 1: Run targeted tests for all touched domains**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test custom_functions_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test capital_structure_integration
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test waterfall_tests
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_corporate
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements --test analysis_scenario_set
```

**Step 2: Run full crate verification**

Run:

```bash
env CARGO_TARGET_DIR=/tmp/rfin-statements-remediation cargo test -p finstack-statements
```

Expected: all tests pass.

**Step 3: Run lint diagnostics**

Use IDE lints on touched files and fix any introduced issues.

**Step 4: Prepare summary**

Document:
- behaviors corrected
- tests added
- any residual follow-up work left outside this branch

**Step 5: Commit**

Do not commit unless explicitly requested by the user.
