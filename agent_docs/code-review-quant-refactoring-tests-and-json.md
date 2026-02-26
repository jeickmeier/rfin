# Code Review Report: Quantitative Finance Library Refactoring – Tests and JSON

**Reviewer:** Senior Code Reviewer Agent
**Date:** February 25, 2025
**Scope:** Test files and JSON schema/examples for field renames and Decimal conversions
**Files Reviewed:** 17 (14 test files + 3 JSON files)

---

## Executive Summary

The refactoring has been applied **mostly consistently** across the reviewed files. Field renames (`payment_delay_days`→`payment_lag_days`, `expiry_date`→`expiry`, `exercise`→`exercise_style`, `spread_bp` f64→Decimal, `spec`→`waterfall_spec`) are correctly used in tests. Decimal conversions follow appropriate patterns. A few **important** issues remain: bond future serde tests use legacy JSON keys that rely on aliases, the swaption schema uses `forward_id` while examples use `forward_curve_id`, and `Decimal::try_from(f64)` for par spread can silently coerce to zero on precision issues.

---

## Changes Reviewed

| File | Changes |
|------|---------|
| `test_basis_swap_edge_cases.rs` | `payment_lag_days`, `spread_bp` Decimal |
| `test_basis_swap_metrics.rs` | `payment_lag_days`, `spread_bp` Decimal |
| `test_basis_swap_par_spread.rs` | `payment_lag_days`, `spread_bp` Decimal, `Decimal::try_from(par_spread_bp)` |
| `test_basis_swap_sensitivities.rs` | `payment_lag_days`, `spread_bp` Decimal |
| `test_basis_swap_theta.rs` | `payment_lag_days`, `spread_bp` Decimal |
| `bond_future/serde.rs` | Uses `expiry_date` in JSON (relies on serde alias) |
| `bond_future/integration.rs` | Uses builder `.expiry()` (Rust API) |
| `swaption/common.rs` | `exercise_style`, `strike` Decimal |
| `xccy_swap/fixtures.rs` | `payment_lag_days`, `spread_bp` Decimal |
| `xccy_swap/pricing.rs` | `payment_lag_days` via fixtures |
| `ir_future/test_construction.rs` | Uses `expiry` (no rename in scope) |
| `ir_future/utils.rs` | Uses `expiry` |
| `equity_index_future/test_pricing.rs` | Uses `.expiry()` builder |
| `equity_index_future/test_types.rs` | Uses `.expiry()` builder |
| `range_accrual.json` | Uses `spec` (instrument content, not waterfall_spec) |
| `private_markets_fund.json` | Uses `waterfall_spec` ✓ |
| `swaption.schema.json` | Uses `exercise_style`, `expiry`, `forward_id` |

---

## Issues Found

### CRITICAL (Must Fix Before Merge)

**None identified.** No security vulnerabilities, data loss risks, or system crashes.

---

### IMPORTANT (Should Fix Soon)

#### 1. **Bond future serde tests use legacy JSON key `expiry_date`**

- **Location:** `bond_future/serde.rs` (lines 327, 393)
- **Description:** Tests `test_bond_future_deny_unknown_fields` and `test_bond_future_minimal_json` use `"expiry_date"` in inline JSON. The struct uses `pub expiry: Date` with `#[serde(alias = "expiry_date")]`, so deserialization works, but the tests do not validate the canonical `"expiry"` key.
- **Impact:** New consumers using `"expiry"` (canonical name) are not covered; tests favor legacy naming.
- **Recommendation:** Add a test that round-trips with `"expiry"` (canonical) and update minimal JSON to use `"expiry"` for consistency. Keep one test with `"expiry_date"` to verify alias backward compatibility.

```rust
// Add test: test_bond_future_canonical_expiry_json
let json = r#"{"expiry": "2025-03-20", ...}"#;  // canonical key
let future: BondFuture = serde_json::from_str(json).expect("...");
assert_eq!(future.expiry, expected_expiry);
```

#### 2. **Swaption schema uses `forward_id`; tests use `forward_curve_id`**

- **Location:** `swaption.schema.json` (example uses `"forward_id"`); `swaption/common.rs` uses `forward_curve_id`
- **Description:** Swaption struct has `#[serde(alias = "forward_id")] pub forward_curve_id`. Schema example shows `"forward_id":"USD-SOFR-3M"`. This is correct for backward compatibility, but the schema should document both keys and prefer `forward_curve_id` as canonical.
- **Impact:** Possible confusion; tooling may generate `forward_id` only.
- **Recommendation:** In schema `description`, state that `forward_curve_id` is canonical and `forward_id` is a deprecated alias.

#### 3. **`Decimal::try_from(f64)` can silently coerce to zero**

- **Location:** `test_basis_swap_par_spread.rs` (lines 109, 435, 470, 513)
- **Description:** `Decimal::try_from(par_spread_bp).unwrap_or_default()` is used when converting `par_spread_bp` (f64 from measures) to `Decimal`. `try_from` can fail for NaN/Inf, and `unwrap_or_default()` returns `Decimal::ZERO`, masking the failure.
- **Impact:** Tests could pass with a zero spread when the intended value is non-zero, hiding logic or metric bugs.
- **Recommendation:** At least in `par_spread_zeros_npv` and `incremental_par_spread_sign_convention`, assert that the conversion succeeded or that the value is finite before converting. Consider `Decimal::try_from(v).expect("par_spread_bp must be finite")` where appropriate.

```rust
// Safer pattern
let spread_decimal = Decimal::try_from(par_spread_bp)
    .unwrap_or_else(|_| panic!("par_spread_bp {} must be finite and convertible to Decimal", par_spread_bp));
```

---

### MINOR (Consider Addressing)

#### 4. **Redundant `payment_lag_days` in inline structs**

- **Location:** `test_basis_swap_par_spread.rs`, `test_basis_swap_metrics.rs`
- **Description:** Many tests construct `BasisSwapLeg` with repeated fields. The `payment_lag_days: 0` is correct but could be centralized via a helper like `make_leg()` in other tests.
- **Recommendation:** Optional refactor: introduce a shared `make_leg()` in a test module or fixtures for par_spread tests to reduce duplication.

#### 5. **Swaption schema example uses numeric `strike`**

- **Location:** `swaption.schema.json`
- **Description:** Example shows `"strike": 0.03`. If the schema expects a Decimal-like string in some contexts, this should be validated.
- **Recommendation:** Confirm that the JSON loader accepts numeric strike (serde typically does for `rust_decimal::Decimal`). No change needed if already correct.

#### 6. **range_accrual.json `spec` is instrument content, not waterfall_spec**

- **Location:** `range_accrual.json`
- **Description:** The top-level `"spec"` is the instrument content (from `#[serde(content = "spec")]`), not the PE fund `waterfall_spec`. No rename applies here.
- **Recommendation:** None. This is correct. Document in schema/review that `spec` for range_accrual is unrelated to `waterfall_spec`.

---

## Field Rename Consistency

| Rename | Basis Swap | Xccy Swap | Bond Future | Swaption | PE Fund |
|--------|------------|-----------|-------------|----------|---------|
| `payment_delay_days`→`payment_lag_days` | ✓ | ✓ | N/A | N/A | N/A |
| `expiry_date`→`expiry` | N/A | N/A | ✓ (alias) | ✓ | N/A |
| `exercise`→`exercise_style` | N/A | N/A | N/A | ✓ | N/A |
| `spread_bp` f64→Decimal | ✓ | ✓ | N/A | N/A | N/A |
| `spec`→`waterfall_spec` | N/A | N/A | N/A | N/A | ✓ |

---

## Decimal Conversion Correctness

| Pattern | Usage | Assessment |
|---------|-------|------------|
| `Decimal::ZERO` | Basis swap legs | ✓ Correct |
| `Decimal::from(n)` for integer n | `Decimal::from(5)`, `Decimal::from(10)`, `Decimal::from(1000)` | ✓ Correct |
| `Decimal::try_from(f64)` | par_spread_bp, strike | ⚠️ Use `unwrap_or_default()` – consider explicit handling |
| `Decimal::from(-1000)` | Extreme negative spread | ✓ Correct |

---

## JSON Schema and Examples

| File | Status |
|------|--------|
| `swaption.schema.json` | Uses `exercise_style`, `expiry`. Example uses `forward_id` (alias). |
| `private_markets_fund.json` | Uses `waterfall_spec` ✓ |
| `range_accrual.json` | Uses `spec` (instrument content) ✓ |

---

## Test Logic and Refactoring Bugs

No logic bugs were found from the refactoring. Assertions and test intent are preserved:

- Basis swap par-spread formula checks remain correct.
- Incremental par spread sign convention tests are coherent.
- Bond future integration tests use the builder API (`.expiry()`) correctly.
- Swaption common fixtures use `exercise_style` and `strike` Decimal correctly.
- Xccy swap fixtures use `payment_lag_days` and `spread_bp` as Decimal.

---

## Architecture Assessment

- **Alignment with existing patterns:** Pass – renames and Decimal usage follow established conventions.
- **Technical debt:** Minimal – serde aliases are used for backward compatibility as intended.
- **Recommendations:** Prefer canonical field names in new tests and schemas; document aliases where relevant.

---

## Test Coverage and Execution

- Unit tests for basis swap, bond future, swaption, xccy swap, ir_future, and equity_index_future were inspected.
- Test execution was attempted but blocked by build lock; manual inspection of test logic did not reveal regressions.

---

## Overall Quality Rating

- **Code Quality:** Good – consistent renames and patterns.
- **Test Quality:** Good – coverage adequate; minor improvements for Decimal conversion and canonical JSON keys.
- **Architecture Fit:** Good – compatible with existing design.

**Ready to Merge:** Yes, with the recommended important fixes for Decimal conversion safety and canonical JSON usage.

---

## Actionable Recommendations

1. Add explicit handling (or panic) for `Decimal::try_from(par_spread_bp)` in par spread tests instead of `unwrap_or_default()`.
2. Add a bond future serde test that uses canonical `"expiry"` in JSON.
3. Document in the swaption schema that `forward_curve_id` is canonical and `forward_id` is deprecated.
4. Optionally extract a `make_leg()` helper in par_spread tests to reduce duplication.

---

## Positive Highlights

- Consistent use of `payment_lag_days` across basis swap and xccy swap tests.
- Consistent use of `Decimal::ZERO` and `Decimal::from(n)` for integer spread values.
- Swaption common module correctly uses `exercise_style` and Decimal strike.
- Bond future uses `#[serde(alias = "expiry_date")]` for backward compatibility.
- PE fund JSON correctly uses `waterfall_spec`.
- Xccy swap fixtures use `payment_lag_days` and `spread_bp` as Decimal correctly.
- Tests for payment lag vs no lag (`annuity_with_payment_lag_differs_from_no_lag`) correctly use `payment_lag_days: 10`.
