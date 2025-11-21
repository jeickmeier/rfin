# Rust Package Code-Quality Review

## 0) Context

* **Crate(s):** `finstack-valuations`
* **Scope of review:** `src/calibration` module
* **Edition / MSRV:** 2021
* **Target(s):** Library
* **Key features/flags:** `strict_validation`
* **Intended users:** Library consumers, quant developers

---

## 1) Executive Summary

* **Overall quality:** **Excellent**. The calibration module is well-architected with clear separation of concerns (config, traits, methods, validation, reporting). It features robust error handling, comprehensive validation logic, and stable serialization schemas.
* **Top risks:**
    *   Hardcoded validation thresholds in `validation.rs` (e.g., -2% floor for forward rates) could trigger false positives in extreme market conditions if not configurable.
    *   String-based index matching in `RatesQuote::is_ois_suitable` is potentially brittle.
* **Quick wins:**
    *   Refactor hardcoded validation constants into `ValidationConfig` or `const` definitions with documentation.
    *   Expand test coverage for `bracket_solve_1d` edge cases.
* **Recommended follow-ups (ranked):**
    1.  Move hardcoded validation bounds to `ValidationConfig`.
    2.  Replace string matching in `is_ois_suitable` with a robust enum or configuration-based approach.

---

## 2) Quick Triage

### Build, Lints, Tests

*   **Build**: Initially failed due to missing/renamed functions in `metrics/sensitivities/cs01.rs`. **Fixed** by updating usages and exports to match the current API (`compute_parallel_cs01_with_context`, `compute_key_rate_cs01_series_with_context`).
*   **Lints**: `cargo clippy --package finstack-valuations` is clean (after fix).
*   **Tests**: `cargo test --package finstack-valuations calibration` passed (60 tests).

### Formatting & Editions

*   Code is well-formatted and consistent.

### Dependency Hygiene

*   Uses standard `serde` for serialization.
*   `finstack_core` dependencies are used idiomatically.

---

## 3) Deep Dive Checklist

### A) Public API & SemVer

*   **Stability**: `Calibrator` trait provides a consistent interface. `CalibrationEnvelope` and `CalibrationSpec` enforce a stable JSON schema (`CALIBRATION_SCHEMA_V1`).
*   **Docs**: Excellent module-level documentation in `mod.rs` and `traits.rs` with examples.
*   **Discoverability**: Re-exports in `mod.rs` make key types accessible.

### B) Error Handling

*   **Typed Errors**: Uses `finstack_core::Result` and `Error`. `ValidationError` provides detailed context (constraint, location, values).
*   **Validation**: `CurveValidator` and `SurfaceValidator` traits provide rigorous checks (no-arbitrage, monotonicity, bounds).
*   **No Panics**: Logic uses `Result` propagation; `unwrap` usage is minimal/absent in logic paths checked.

### C) Concurrency, Mutability, and Safety

*   **Concurrency**: `CalibrationConfig` has a `use_parallel` flag, indicating readiness for parallel execution.
*   **Safety**: No `unsafe` blocks observed in the reviewed files.

### D) Performance & Allocations

*   **Allocations**: `validation.rs` uses `const` arrays (e.g., `DF_MONO_POINTS`) for validation points to avoid repeated allocations.
*   **Efficiency**: Solver wrappers (`solve_1d`, `bracket_solve_1d`) are lightweight.

### E) Correctness & Testing

*   **Validation**: Extensive logic to catch financial inconsistencies (e.g., negative forward rates, calendar arbitrage).
*   **Tests**: `tests/calibration.rs` and inline tests cover serialization, round-trips, and basic logic.

### F) Security

*   **Input Validation**: `CalibrationEnvelope` relies on `serde_json` with `deny_unknown_fields`, which is good practice.
*   **DoS Prevention**: No obvious unbounded recursion or allocation loops.

---

## 5) Structured Findings

| ID   | Area           | Severity | Finding                                                                 | Evidence              | Recommendation                                            | Effort |
| ---- | -------------- | -------- | ----------------------------------------------------------------------- | --------------------- | --------------------------------------------------------- | ------ |
| F-01 | Correctness    | Medium   | Hardcoded validation thresholds (e.g. -0.02 min fwd rate)               | `validation.rs:110`   | Move these constants to `ValidationConfig`                | S      |
| F-02 | Robustness     | Medium   | Brittle string matching for OIS detection                               | `quote.rs:105`        | Use `IndexId` or config-based matching                    | S      |
| F-03 | Build          | High     | Broken compilation in `metrics` (unrelated but blocking)                | `metrics/mod.rs`      | **FIXED**: Updated exports and usage in `cs01.rs`         | Done   |
| F-04 | Observability  | Low      | `tracing::warn!` used for non-strict validation failures                | `validation.rs:250`   | Consider collecting warnings in `CalibrationReport` too   | M      |

---

## 9) Follow-Up Plan

1.  **(Completed)** Fixed compilation errors in `finstack/valuations/src/metrics` to restore build health.
2.  **(High)** Refactor `CurveValidator` implementations to use thresholds from `ValidationConfig` where possible, or strictly defined constants.
3.  **(Medium)** Review `RatesQuote::is_ois_suitable` to ensure it handles all necessary OIS index variants reliably.

