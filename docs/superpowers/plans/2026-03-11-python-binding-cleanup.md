# Python Binding Cleanup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the binding-layer drift identified in the Python binding review by moving composite analytics logic into Rust core, normalizing the tornado entry wrapper pattern, adding regression tests, and removing stale lint/documentation suppressions.

**Architecture:** Keep Rust core as the single source of truth for financial calculations and derived analytics. Python bindings should only convert types, delegate to core helpers, wrap core structs, and map errors to Python exceptions. Add targeted Python tests around the touched public APIs so the cleanup is locked in without broad refactoring.

**Tech Stack:** Rust, PyO3, pytest, uv, Polars

---

## Chunk 1: Tornado Wrapper Parity

### Task 1: Add failing tornado generation tests

**Files:**
- Modify: `finstack-py/tests/test_statements_parity.py`
- Test: `finstack-py/tests/test_statements_parity.py`

- [ ] **Step 1: Write the failing test**

```python
def test_generate_tornado_chart_sorts_by_swing() -> None:
    builder = ModelBuilder.new("tornado_test")
    builder.periods("2025Q1..Q1", None)
    builder.value("revenue", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0))])
    builder.value("cost", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(40000.0))])
    builder.compute("gross_profit", "revenue - cost")
    model = builder.build()

    analyzer = SensitivityAnalyzer(model)
    config = SensitivityConfig(SensitivityMode.DIAGONAL)
    config.add_parameter(ParameterSpec.with_percentages("revenue", PeriodId.quarter(2025, 1), 100000.0, [-10.0, 0.0, 10.0]))
    config.add_parameter(ParameterSpec.with_percentages("cost", PeriodId.quarter(2025, 1), 40000.0, [-25.0, 0.0, 25.0]))
    config.add_target_metric("gross_profit")

    result = analyzer.run(config)
    entries = generate_tornado_chart(result, "gross_profit@2025Q1")

    assert [entry.parameter_id for entry in entries] == ["revenue", "cost"]
    assert entries[0].swing >= entries[1].swing
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest finstack-py/tests/test_statements_parity.py -k tornado -v`
Expected: FAIL because the current binding wrapper/drift or ordering behavior is not fully exercised yet.

- [ ] **Step 3: Write minimal implementation**

Implement `PyTornadoEntry` as an `inner: TornadoEntry` wrapper with `from_inner()` and delegate getters / `swing()` to the core type.

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest finstack-py/tests/test_statements_parity.py -k tornado -v`
Expected: PASS

### Task 2: Refactor tornado binding wrapper

**Files:**
- Modify: `finstack-py/src/statements/analysis/mod.rs`
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`

- [ ] **Step 1: Replace duplicated tornado fields with inner wrapper**
- [ ] **Step 2: Add `from_inner()` helper and delegate getters / `swing()`**
- [ ] **Step 3: Keep Python constructor behavior stable**
- [ ] **Step 4: Update typing/docstrings if needed**

## Chunk 2: Core Analytics Delegation

### Task 3: Add failing expr-plugin parity tests for composite metrics

**Files:**
- Modify: `finstack-py/tests/parity/test_expr_plugin_parity.py`
- Test: `finstack-py/tests/parity/test_expr_plugin_parity.py`

- [ ] **Step 1: Write failing tests**

Add focused parity tests for `risk_of_ruin`, `recovery_factor`, `martin_ratio`, `sterling_ratio`, `pain_ratio`, and `m_squared`.

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest finstack-py/tests/parity/test_expr_plugin_parity.py -k "risk_of_ruin or recovery_factor or martin_ratio or sterling_ratio or pain_ratio or m_squared" -v`
Expected: FAIL if any new core helper or binding delegation path is missing.

- [ ] **Step 3: Write minimal implementation**

Create core helper functions in `finstack/core/src/analytics/risk_metrics.rs` for the composite calculations and update `finstack-py/src/core/analytics/expr_plugin.rs` to delegate directly.

- [ ] **Step 4: Run test to verify it passes**

Run the same command again and confirm PASS.

### Task 4: Move composite analytics into Rust core

**Files:**
- Modify: `finstack/core/src/analytics/risk_metrics.rs`
- Modify: `finstack/core/src/analytics/mod.rs`
- Modify: `finstack-py/src/core/analytics/expr_plugin.rs`

- [ ] **Step 1: Add small core helpers for the remaining composite metrics**
- [ ] **Step 2: Export the helpers from the analytics module**
- [ ] **Step 3: Replace binding-side multi-step calculations with single core calls**
- [ ] **Step 4: Keep signatures minimal and avoid adding new public surface beyond needed helpers**

## Chunk 3: Binding Validation and Hygiene

### Task 5: Add failing regression tests for binding validation paths

**Files:**
- Modify: `finstack-py/tests/test_error_handling.py`
- Test: `finstack-py/tests/test_error_handling.py`

- [ ] **Step 1: Write failing tests**

Add targeted tests for:
- `CDSTranche.builder(...).attach_pct(...).detach_pct(...).build()` invalid tranche structure
- `CDSOption.builder(...).build()` missing required fields raising a descriptive error

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest finstack-py/tests/test_error_handling.py -k "tranche or cds_option" -v`
Expected: FAIL before cleanup if exact regression behavior is not covered yet.

- [ ] **Step 3: Write minimal implementation**

Preserve the current validation/error mapping behavior while removing stale suppression attributes and keeping messages explicit.

- [ ] **Step 4: Run tests to verify they pass**

Run the same command again and confirm PASS.

### Task 6: Remove stale suppression/docs

**Files:**
- Modify: `finstack-py/src/core/math/linalg.rs`
- Modify: `finstack-py/src/valuations/instruments/credit_derivatives/cds_option.rs`

- [ ] **Step 1: Remove the stale `# Panics` note and `#[allow(clippy::expect_used)]`**
- [ ] **Step 2: Remove the stale `#[allow(clippy::unwrap_used)]` from `PyCDSOptionBuilder::build()`**
- [ ] **Step 3: Re-read touched functions to confirm behavior did not change**

## Chunk 4: Verification

### Task 7: Run focused verification

**Files:**
- Test: `finstack-py/tests/test_statements_parity.py`
- Test: `finstack-py/tests/parity/test_expr_plugin_parity.py`
- Test: `finstack-py/tests/test_error_handling.py`

- [ ] **Step 1: Run tornado tests**

Run: `uv run pytest finstack-py/tests/test_statements_parity.py -k tornado -v`

- [ ] **Step 2: Run expr-plugin parity tests**

Run: `uv run pytest finstack-py/tests/parity/test_expr_plugin_parity.py -k "risk_of_ruin or recovery_factor or martin_ratio or sterling_ratio or pain_ratio or m_squared" -v`

- [ ] **Step 3: Run error-handling regression tests**

Run: `uv run pytest finstack-py/tests/test_error_handling.py -k "tranche or cds_option" -v`

- [ ] **Step 4: Run lints on touched files**

Use the editor lints plus any project-local checks required for the touched Rust and Python binding files.

- [ ] **Step 5: Review diffs for scope control**

Confirm only the intended binding/core cleanup and regression tests were changed.
