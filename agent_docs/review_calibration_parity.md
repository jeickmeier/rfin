# Code Review: Python Calibration Bindings — Full Parity with Rust Crate

**Reviewer:** Senior Code Reviewer Agent
**Date:** 2026-03-01
**Files Reviewed:** 11
**Scope:** PyO3 bindings + .pyi stubs for `finstack_valuations::calibration`

## Executive Summary

This is a high-quality set of changes that brings the Python calibration bindings to near-complete parity with the Rust crate. The code consistently follows the project's established binding patterns, keeps all math/logic in Rust, handles errors properly, and provides comprehensive `.pyi` stubs. There are no critical issues. I found a few minor gaps and one important inconsistency worth addressing before merge.

## Changes Reviewed

| File | Description |
|---|---|
| `finstack-py/src/valuations/calibration/config.rs` | 6 new `#[pyclass]` types + extended CalibrationConfig/RateBounds |
| `finstack-py/src/valuations/calibration/report.rs` | 4 new getters + updated to_dict() |
| `finstack-py/src/valuations/calibration/validation.rs` | 3 presets + builder + validate_base_correlation_curve |
| `finstack-py/src/valuations/calibration/sabr.rs` | SABRMarketData.with_shift + SABRCalibrationDerivatives.with_fd |
| `finstack-py/src/valuations/bumps.rs` | bump_discount_curve function |
| `finstack-py/finstack/valuations/calibration/config.pyi` | Stubs for all new config types |
| `finstack-py/finstack/valuations/calibration/validation.pyi` | Stubs for presets + new function |
| `finstack-py/finstack/valuations/calibration/report.pyi` | Stubs for new report getters |
| `finstack-py/finstack/valuations/calibration/sabr.pyi` | Stubs for with_shift + with_fd |
| `finstack-py/finstack/valuations/calibration/__init__.pyi` | Updated re-exports + **all** |
| `finstack-py/finstack/valuations/bumps.pyi` | Stubs for bump_discount_curve |

---

## Issues Found

### IMPORTANT (Should Address Before Merge)

#### 1. Missing `shift` property getter on `SABRMarketData`

- **Location:** `finstack-py/src/valuations/calibration/sabr.rs`
- **Description:** The Rust `SABRMarketData` struct has `pub shift: Option<f64>` and the Python binding exposes a `with_shift()` constructor. However, there is no `#[getter]` for `shift`, so users who create shifted SABR data cannot read back the shift value.
- **Impact:** Incomplete round-trip. Users can set but not inspect shift, which breaks introspection patterns like `repr` display and debugging.
- **Recommendation:**

  In `sabr.rs`, add a getter alongside the other getters:

  ```rust
  #[getter]
  fn shift(&self) -> Option<f64> {
      self.inner.shift
  }
  ```

  In `sabr.pyi`, add:

  ```python
  @property
  def shift(self) -> float | None: ...
  ```

  Also consider updating `__repr__` to include shift when present:

  ```rust
  fn __repr__(&self) -> String {
      let shift_str = self.inner.shift
          .map(|s| format!(", shift={:.4}", s))
          .unwrap_or_default();
      format!(
          "SABRMarketData(forward={:.2}, time_to_expiry={:.2}, strikes={}, beta={:.2}{})",
          self.inner.forward, self.inner.time_to_expiry,
          self.inner.strikes.len(), self.inner.beta, shift_str
      )
  }
  ```

#### 2. GIL release inconsistency in `bumps.rs`

- **Location:** `finstack-py/src/valuations/bumps.rs`
- **Description:** `py_bump_discount_curve` (line 318) correctly uses `py.detach()` to release the GIL during CPU-heavy calibration. However, the four other bump functions (`bump_discount_curve_synthetic`, `bump_hazard_spreads`, `bump_hazard_shift`, `bump_inflation_rates`) do NOT release the GIL, even though they perform equally expensive Rust calibration work.
- **Impact:** Python threads are blocked during calibration for 4 out of 5 bump functions, reducing throughput in multi-threaded scenarios.
- **Recommendation:** Wrap all bump function bodies with `py.detach()` for consistency. Example for `py_bump_discount_curve_synthetic`:

  ```rust
  fn py_bump_discount_curve_synthetic(
      py: Python<'_>,  // add py parameter
      curve: &PyDiscountCurve,
      market: &PyMarketContext,
      bump: &PyBumpRequest,
      as_of: &Bound<'_, PyAny>,
  ) -> PyResult<PyDiscountCurve> {
      let as_of_date = py_to_date(as_of)?;
      let curve_inner = curve.inner.clone();
      let market_inner = market.inner.clone();
      let bump_inner = bump.inner.clone();
      let bumped = py
          .detach(|| bump_discount_curve_synthetic(&curve_inner, &market_inner, &bump_inner, as_of_date))
          .map_err(core_to_py)?;
      Ok(PyDiscountCurve::new_arc(Arc::new(bumped)))
  }
  ```

  Note: `py.detach()` requires all captured data to be `Send`. The `Arc`-wrapped inner types should satisfy this, but verify before applying across all functions. If any inner type is `!Send`, the current approach without GIL release is correct and the inconsistency should be documented.

### MINOR (Consider Addressing)

#### 3. `ValidationConfig` missing `butterfly_upper_ratio` / `butterfly_lower_ratio`

- **Location:** `finstack-py/src/valuations/calibration/validation.rs` + `.pyi`
- **Description:** The Rust `ValidationConfig` has two additional fields (`butterfly_upper_ratio: f64` and `butterfly_lower_ratio: f64`) with serde defaults that are not exposed in the Python binding — neither as constructor arguments nor as property getters.
- **Impact:** Users who need to tune butterfly spread convexity tolerances cannot do so from Python. The Rust defaults (1.25 / 0.75) will always apply.
- **Recommendation:** If these are considered advanced/internal, this is acceptable. If full parity is the goal, add them to both the constructor and as getters. Given the stated goal of 100% parity, these should be exposed.

#### 4. `.pyi` stubs missing `__hash__` and `__eq__` on `CalibrationMethod`

- **Location:** `finstack-py/finstack/valuations/calibration/config.pyi`
- **Description:** The Rust binding for `CalibrationMethod` implements `__hash__` and `__richcmp__` (providing `__eq__` behavior), but the `.pyi` stub only declares `__repr__`. Similarly, `ValidationMode` is missing `__hash__` in the stub.
- **Impact:** IDE tooling (mypy, pyright) won't know these types are hashable/comparable, which affects usage in sets and dict keys.
- **Recommendation:** Add to `config.pyi`:

  For `CalibrationMethod`:

  ```python
  def __str__(self) -> str: ...
  def __hash__(self) -> int: ...
  ```

  For `ValidationMode`:

  ```python
  def __hash__(self) -> int: ...
  ```

#### 5. `CalibrationConfig.__init__` keyword-only mismatch between stub and runtime

- **Location:** `config.pyi` line 199 vs `config.rs` line 931
- **Description:** The `.pyi` stub declares `__init__(self, *, tolerance=..., ...)` with `*` (keyword-only), but the PyO3 signature `#[pyo3(signature = (tolerance=None, ...))]` allows positional arguments at runtime. A user could technically call `CalibrationConfig(1e-8, 100, True)` at runtime but mypy/pyright would reject it.
- **Impact:** The stub is more restrictive than reality. This is a deliberate "API guidance" pattern (14 positional args would be terrible), so this is acceptable. Document intent if challenged.
- **Recommendation:** No change needed. The stub correctly guides usage. If you want strict alignment, add a bare `*` to the PyO3 signature by putting `/` and `*` markers, though PyO3's support for this is limited.

### NIT (Informational)

#### 6. `to_dict()` in `CalibrationReport` omits `solver_config`

- **Location:** `finstack-py/src/valuations/calibration/report.rs:141-158`
- **Description:** The `to_dict()` method was updated to include `validation_passed`, `validation_error`, and `model_version`, but does not include `solver_config`. All other new fields are included.
- **Impact:** Round-tripping `report.to_dict()` loses solver config information. Users relying on `to_dict()` for serialization/logging won't see which solver was used.
- **Recommendation:** Add `solver_config` to `to_dict()`:

  ```rust
  dict.set_item("solver_config", self.solver_config().name())?;
  ```

  Or for full fidelity, serialize the full solver config as a sub-dict.

#### 7. Intentional parity gaps (acceptable omissions)

The following Rust APIs are correctly NOT exposed in Python since they are internal/engine-facing:
- `CalibrationConfig::from_finstack_config_or_default()` — config plumbing
- `CalibrationConfig::create_lm_solver()` — solver construction detail
- `CalibrationConfig::with_explain_opts(ExplainOpts)` — only enabled/disabled is needed from Python
- `CalibrationReport::new()`, `for_type_with_tolerance()`, `with_*()` builders — `CalibrationReport` is an output type, not user-constructed

---

## Architecture Assessment

- **Alignment with existing patterns:** Pass. All new types follow the established `Py<X> { inner: X }` wrapper pattern with `#[pyclass(frozen, from_py_object)]`.
- **Thin binding layer:** Pass. Zero math or business logic in the binding layer. All presets/builders delegate directly to Rust.
- **Module registration:** Pass. All new types are properly registered in `config::register()` and included in the export list.
- **Technical debt introduced:** None.

## Test Coverage Analysis

- **Unit tests:** The underlying Rust types have tests (e.g., `CalibrationReport::for_type_with_tolerance` tests in `report.rs`). Binding-level tests were not reviewed but the thin-wrapper pattern means Rust-level tests provide high confidence.
- **Missing test scenarios for bindings:**
  - Round-trip test: construct each new type in Python, read back all properties, verify values match
  - Preset tests: `ValidationConfig.strict()`, `.negative_rates()`, `.lenient()` should be verified from Python
  - `bump_discount_curve` with invalid `params` dict should produce a clear `ValueError`
  - `SABRMarketData.with_shift` with negative shift should raise `ValueError`

## Performance Considerations

- The GIL release inconsistency (Issue #2) means 4 out of 5 bump functions hold the GIL during potentially expensive calibration. This should be fixed for production multi-threaded usage.
- `CalibrationConfig.__repr__` creates several intermediate `Py*` wrapper objects just for `.name()` calls. This is trivial overhead but could be simplified by matching directly on inner discriminants.

## Security Assessment

No security concerns. The binding layer performs no I/O, no file access, and no network calls. Input validation is delegated to Rust.

## Overall Quality Rating

| Dimension | Rating |
|---|---|
| Code Quality | Excellent |
| Pattern Consistency | Excellent |
| Architecture Fit | Excellent |
| Error Handling | Excellent |
| Stub Completeness | Good (minor gaps) |
| Ready to Merge | Yes, with minor fixes |

## Actionable Recommendations (Priority Order)

1. **Add `shift` getter to `SABRMarketData`** — straightforward, ensures round-trip introspection
2. **Make GIL release consistent across all bump functions** — important for production perf
3. **Add `butterfly_upper_ratio` / `butterfly_lower_ratio` to `ValidationConfig`** — if 100% parity is the goal
4. **Add missing `__hash__`/`__eq__` to `.pyi` stubs** for `CalibrationMethod` and `ValidationMode`
5. **Add `solver_config` to `CalibrationReport.to_dict()`** — completeness
6. **Consider Python-level round-trip tests** for new types

## Positive Highlights

- **Excellent binding pattern discipline.** Every new type faithfully follows the project's Py wrapper convention with frozen classes, from_py_object, proper error handling, and consistent `__repr__`/`__hash__`/`__richcmp__`.
- **Zero logic leakage.** All presets (`conservative`, `aggressive`, `fast`, `distressed`, `strict`, `negative_rates`, `lenient`) delegate directly to the Rust implementations. No constants, no formulas, no business logic in the binding layer.
- **Thoughtful CalibrationConfig constructor ordering.** The comment at lines 968-971 explaining why solver_kind is applied before tolerance/max_iterations demonstrates careful API design.
- **Comprehensive `.pyi` stubs.** The stubs match the Rust implementations closely and provide good type hints for IDE tooling. The `__init__.pyi` properly re-exports everything with a complete `__all__` list.
- **Clean error propagation.** Consistent use of `map_err(core_to_py)` and `PyKeyError`/`PyValueError` for validation errors.
