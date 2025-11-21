# Rust Package Code-Quality Review

## 0) Context

* **Crate(s):** `finstack-valuations` (specifically `cashflow` module)
* **Scope of review:** Module `src/cashflow` and submodules
* **Edition / MSRV:** 2021 / 1.74+ (inferred from workspace standards)
* **Target(s):** Library (lib), WASM compatible (pure Rust logic), Python bindings
* **Key features/flags:** `serde` (optional)
* **Intended users:** Library consumers, internal instrument implementations

---

## 1) Executive Summary

* **Overall quality:** **High**. The code is well-documented, strictly typed, and currency-safe. It follows the workspace's deterministic and "correctness first" philosophy.
* **Top risks:**
  * **Panics in Builder:** The `CashflowBuilder` has methods (`fixed_cf`, `floating_cf`) that panic if `principal()` hasn't been called first. While `try_` variants exist, the panicking versions are easy to misuse.
  * **Error Swallowing in Deprecated API:** `aggregation::pv_by_period` swallows errors via `unwrap_or_default()` to maintain legacy behavior.
* **Quick wins:**
  * Deprecate the panicking builder methods or change them to return `Result`.
  * Standardize epsilon usage (currently hardcoded `1e-12` in `accrual.rs`).
* **Recommended follow-ups:**
  1. Remove deprecated error-swallowing functions in the next major version.
  2. Audit `CashflowBuilder` usage to ensure `try_` variants are preferred.

---

## 2) Quick Triage

### Build, Lints, Tests

* **Lints:** `cargo clippy` passes with no warnings (checked).
* **Tests:** `cargo test` passes (36 tests in `cashflow` module).
* **CI:** Assumed standard workspace CI.

### Formatting & Editions

* **Formatting:** Consistent, standard Rust formatting.
* **Edition:** 2021.

### Dependency Hygiene

* **Dependencies:** Uses `finstack-core`, `time`, `indexmap`, `smallvec`, `hashbrown`. These are standard and appropriate.

---

## 3) Deep Dive Checklist

### A) Public API & SemVer

* **Stability:** Public API is well-structured with re-exports in `mod.rs`.
* **Discoverability:** Good use of `mod.rs` to expose `builder`, `aggregation`, `traits`.
* **Docs:** Excellent documentation with examples in `mod.rs`, `builder/mod.rs`.
* **SemVer:** Deprecated functions are clearly marked with `#[deprecated]`.

### B) Error Handling

* **Pattern:** Uses `finstack_core::Result` and specific error variants (`InputError`, `CurrencyMismatch`).
* **Issues:**
  * `CashflowBuilder` methods (`fixed_cf`, etc.) panic if state is invalid.
  * `aggregation::pv_by_period` uses `unwrap_or_default()`, swallowing errors.

### C) Concurrency, Mutability, and Safety

* **Concurrency:** `CashflowProvider` requires `Send + Sync`. `Arc` used for curves.
* **Safety:** No `unsafe` blocks found in reviewed files.
* **Mutability:** Builder pattern uses mutable borrows `&mut Self`, standard for builders.

### D) Performance & Allocations

* **Hot Paths:** `aggregation` uses `sort_unstable_by_key` and avoids `inline(never)`.
* **Allocations:** `SmallVec` used in builder for fees (`SmallVec<[FeeSpec; 4]>`) to optimize for common cases. `hashbrown` used for maps.
* **Zero-copy:** Iterators used effectively in aggregation.

### E) Correctness & Testing

* **Determinism:** Explicitly designed for determinism (e.g., sorting flows before aggregation).
* **Currency Safety:** Strong enforcement. `aggregation::aggregate_cashflows_precise_checked` errors on mismatch.
* **Tests:** Unit tests cover core logic, builder emission, and edge cases (zero recovery, etc.).

### F) Security

* **Input Validation:** Builder `validate_core_inputs` ensures necessary fields are present.
* **DoS:** No obvious recursion or unbounded allocation risks (dates are constrained by input schedules).

### G) Packaging & CI/CD

* **Metadata:** Part of workspace.

---

## 5) Structured Findings

| ID   | Area           | Severity | Finding                                                                 | Evidence                               | Recommendation                                            | Effort |
| ---- | -------------- | -------- | ----------------------------------------------------------------------- | -------------------------------------- | --------------------------------------------------------- | ------ |
| F-01 | Error Handling | Medium   | `CashflowBuilder` methods panic if principal not set                    | `builder/builder.rs:379`               | Use `try_` variants or return `Result` in all methods     | M      |
| F-02 | Error Handling | Low      | Deprecated `pv_by_period` swallows errors                               | `aggregation.rs:211`                   | Remove in next major version; warn users now              | S      |
| F-03 | Maintenance    | Low      | Hardcoded epsilon `1e-12` in `accrual.rs`                               | `accrual.rs:352`                       | Move to a constant in `finstack_core` or top of file      | S      |
| F-04 | API            | Low      | `CashflowProvider::build_full_schedule` has heavy default impl          | `traits.rs:42`                         | Ensure default impl is sufficient or mark as required     | S      |

---

## 9) Follow-Up Plan

1. (Medium) **Refactor Builder API**: Transition `CashflowBuilder` to a state-typed builder or consistently return `Result` to avoid panics. The current `try_` vs non-`try` duality is a bit prone to confusion.
2. (Low) **Remove Deprecations**: Schedule removal of `pv_by_period` (the error-swallowing version).
3. (Low) **Epsilon Standardization**: Centralize small number comparisons if used elsewhere.

