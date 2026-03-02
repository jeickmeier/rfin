# Core Python Binding: 100% Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close every remaining gap between the Rust `finstack-core` public API and the `finstack.core` Python bindings, achieving 100% parity.

**Architecture:** Six independent work streams: (1) Analytics Performance facade, (2) Market Data missing curve types, (3) Math advanced types, (4) FX providers + Type IDs, (5) Dates utilities, (6) Cashflow minor methods. Streams can run in parallel since they touch non-overlapping files.

**Tech Stack:** Rust (pyo3 0.28), Python (.pyi type stubs), maturin build system

**Design Doc:** `docs/plans/2026-03-01-core-python-parity-design.md`

---

## Stream 1: Analytics Performance Facade (P0)

### Task 1: Add risk metric methods to Performance

**Files:**
- Modify: `finstack-py/src/core/analytics/performance.rs`

**Step 1: Read the existing Performance binding** to understand what methods exist and the pattern used.

Read: `finstack-py/src/core/analytics/performance.rs` (full file)
Read: `finstack/core/src/analytics/risk_metrics.rs` (full file — source of truth for function signatures)

**Step 2: Add risk metric methods to `#[pymethods] impl PyPerformance`**

Add the following methods, each delegating to the corresponding `finstack_core::analytics::risk_metrics` function. The pattern is:

```rust
#[pyo3(text_signature = "($self, risk_free_rate=0.0)")]
fn sharpe(&self, risk_free_rate: Option<f64>) -> PyResult<f64> {
    let rfr = risk_free_rate.unwrap_or(0.0);
    risk_metrics::sharpe(&self.inner.returns(), rfr)
        .map_err(core_to_py)
}
```

Methods to add (all follow the same delegation pattern):

| Method | Rust Function | Python Signature |
|---|---|---|
| `sharpe` | `risk_metrics::sharpe` | `(risk_free_rate: float = 0.0) -> float` |
| `sortino` | `risk_metrics::sortino` | `(risk_free_rate: float = 0.0, target: float = 0.0) -> float` |
| `volatility` | `risk_metrics::volatility` | `() -> float` |
| `cagr` | `risk_metrics::cagr` | `() -> float` |
| `calmar` | `risk_metrics::calmar` | `() -> float` |
| `value_at_risk` | `risk_metrics::value_at_risk` | `(confidence: float = 0.95) -> float` |
| `parametric_var` | `risk_metrics::parametric_var` | `(confidence: float = 0.95) -> float` |
| `cornish_fisher_var` | `risk_metrics::cornish_fisher_var` | `(confidence: float = 0.95) -> float` |
| `expected_shortfall` | `risk_metrics::expected_shortfall` | `(confidence: float = 0.95) -> float` |
| `omega_ratio` | `risk_metrics::omega_ratio` | `(threshold: float = 0.0) -> float` |
| `kurtosis` | `risk_metrics::kurtosis` | `() -> float` |
| `skewness` | `risk_metrics::skewness` | `() -> float` |
| `downside_deviation` | `risk_metrics::downside_deviation` | `(target: float = 0.0) -> float` |
| `mean_return` | `risk_metrics::mean_return` | `() -> float` |
| `geometric_mean` | `risk_metrics::geometric_mean` | `() -> float` |
| `gain_to_pain` | `risk_metrics::gain_to_pain` | `() -> float` |
| `tail_ratio` | `risk_metrics::tail_ratio` | `() -> float` |
| `ulcer_index` | `risk_metrics::ulcer_index` | `() -> float` |
| `pain_index` | `risk_metrics::pain_index` | `() -> float` |
| `pain_ratio` | `risk_metrics::pain_ratio` | `() -> float` |
| `martin_ratio` | `risk_metrics::martin_ratio` | `() -> float` |
| `burke_ratio` | `risk_metrics::burke_ratio` | `() -> float` |
| `sterling_ratio` | `risk_metrics::sterling_ratio` | `() -> float` |
| `recovery_factor` | `risk_metrics::recovery_factor` | `() -> float` |
| `risk_of_ruin` | `risk_metrics::risk_of_ruin` | `() -> float` |
| `modified_sharpe` | `risk_metrics::modified_sharpe` | `() -> float` |
| `m_squared` | `risk_metrics::m_squared` | `(benchmark_returns: list[float]) -> float` |
| `outlier_win_ratio` | `risk_metrics::outlier_win_ratio` | `() -> float` |
| `outlier_loss_ratio` | `risk_metrics::outlier_loss_ratio` | `() -> float` |

NOTE: Read the Rust source for each function to verify exact parameter types and return types. Some functions may take `&[f64]` returns directly, others may need `PeriodKind` for annualization. Adapt the Python signatures accordingly.

**Step 3: Build and smoke test**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.core.analytics import Performance; print(dir(Performance))" | tr ',' '\n' | grep -E 'sharpe|sortino|volatility'`
Expected: Methods `sharpe`, `sortino`, `volatility` should appear.

**Step 4: Commit**

```bash
git add finstack-py/src/core/analytics/performance.rs
git commit -m "feat(py): add risk metric methods to Performance facade"
```

---

### Task 2: Add rolling metric methods to Performance

**Files:**
- Modify: `finstack-py/src/core/analytics/performance.rs`

**Step 1: Read the rolling metrics source**

Read: `finstack/core/src/analytics/risk_metrics.rs` — search for `rolling_sharpe`, `rolling_sortino`, `rolling_volatility`, `RollingSharpe`, `RollingSortino`, `RollingVolatility`.

**Step 2: Add rolling methods**

```rust
#[pyo3(text_signature = "($self, window)")]
fn rolling_sharpe(&self, window: usize) -> PyResult<Vec<Option<f64>>> {
    let result = risk_metrics::rolling_sharpe(&self.inner.returns(), window)
        .map_err(core_to_py)?;
    Ok(result.values)
}

#[pyo3(text_signature = "($self, window)")]
fn rolling_sortino(&self, window: usize) -> PyResult<Vec<Option<f64>>> {
    let result = risk_metrics::rolling_sortino(&self.inner.returns(), window)
        .map_err(core_to_py)?;
    Ok(result.values)
}

#[pyo3(text_signature = "($self, window)")]
fn rolling_volatility(&self, window: usize) -> PyResult<Vec<Option<f64>>> {
    let result = risk_metrics::rolling_volatility(&self.inner.returns(), window)
        .map_err(core_to_py)?;
    Ok(result.values)
}
```

NOTE: Check the actual return type of the rolling functions. They may return a struct with a `values` field, or a `Vec<Option<f64>>` directly. Adjust accordingly.

**Step 3: Build and test**

Run: `cd finstack-py && maturin develop --release`

**Step 4: Commit**

```bash
git add finstack-py/src/core/analytics/performance.rs
git commit -m "feat(py): add rolling_sharpe, rolling_sortino, rolling_volatility to Performance"
```

---

### Task 3: Add benchmark-relative methods to Performance

**Files:**
- Modify: `finstack-py/src/core/analytics/performance.rs`

**Step 1: Read the benchmark module source**

Read: `finstack/core/src/analytics/benchmark.rs` (full file)

**Step 2: Add benchmark methods** (require `.with_benchmark()` to have been called)

```rust
fn require_benchmark(&self) -> PyResult<&[f64]> {
    self.inner.benchmark_returns()
        .ok_or_else(|| PyValueError::new_err(
            "No benchmark set. Call .with_benchmark() first."
        ))
}

#[pyo3(text_signature = "($self)")]
fn beta(&self) -> PyResult<f64> {
    let bench = self.require_benchmark()?;
    benchmark::calc_beta(&self.inner.returns(), bench)
        .map(|r| r.beta)
        .map_err(core_to_py)
}

#[pyo3(text_signature = "($self)")]
fn alpha(&self) -> PyResult<f64> {
    let bench = self.require_benchmark()?;
    benchmark::calc_beta(&self.inner.returns(), bench)
        .map(|r| r.alpha)
        .map_err(core_to_py)
}
```

Add the following methods (all require benchmark):

| Method | Rust Function | Returns |
|---|---|---|
| `beta` | `benchmark::calc_beta` | `.beta` field |
| `alpha` | `benchmark::calc_beta` | `.alpha` field |
| `r_squared` | `benchmark::r_squared` | `f64` |
| `information_ratio` | `benchmark::information_ratio` | `f64` |
| `tracking_error` | `benchmark::tracking_error` | `f64` |
| `treynor` | `benchmark::treynor` | `f64` |
| `up_capture` | `benchmark::up_capture` | `f64` |
| `down_capture` | `benchmark::down_capture` | `f64` |
| `batting_average` | `benchmark::batting_average` | `f64` |

NOTE: Read the exact function signatures — some may need additional params like `risk_free_rate`. Adapt accordingly.

**Step 3: Build and test**

Run: `cd finstack-py && maturin develop --release`

**Step 4: Commit**

```bash
git add finstack-py/src/core/analytics/performance.rs
git commit -m "feat(py): add benchmark-relative methods to Performance (beta, alpha, info ratio, etc.)"
```

---

### Task 4: Add drawdown and return helper methods to Performance

**Files:**
- Modify: `finstack-py/src/core/analytics/performance.rs`

**Step 1: Read the drawdown and returns modules**

Read: `finstack/core/src/analytics/drawdown.rs`
Read: `finstack/core/src/analytics/returns.rs`
Read: `finstack/core/src/analytics/lookback.rs`
Read: `finstack/core/src/analytics/consecutive.rs`
Read: `finstack/core/src/analytics/aggregation.rs`

**Step 2: Add drawdown methods**

| Method | Rust Function |
|---|---|
| `max_drawdown` | `drawdown::max_drawdown` or extract from `drawdown_details` |
| `max_drawdown_duration` | `drawdown::max_drawdown_duration` |
| `avg_drawdown` | `drawdown::avg_drawdown` |
| `cdar` | `drawdown::cdar` |
| `drawdown_series` | `drawdown::to_drawdown_series` |

**Step 3: Add return helper methods**

| Method | Rust Function |
|---|---|
| `cumulative_return` | `returns::comp_total` |
| `excess_returns` | `returns::excess_returns` |

**Step 4: Add lookback, aggregation, consecutive as static methods**

| Method | Rust Function |
|---|---|
| `@staticmethod ytd_select` | `lookback::ytd_select` |
| `@staticmethod qtd_select` | `lookback::qtd_select` |
| `@staticmethod mtd_select` | `lookback::mtd_select` |
| `@staticmethod fytd_select` | `lookback::fytd_select` |
| `@staticmethod group_by_period` | `aggregation::group_by_period` |
| `@staticmethod count_consecutive` | `consecutive::count_consecutive` |

NOTE: These static methods don't need `self`. They operate on raw data passed as lists.

**Step 5: Build and test**

Run: `cd finstack-py && maturin develop --release`

**Step 6: Commit**

```bash
git add finstack-py/src/core/analytics/performance.rs
git commit -m "feat(py): add drawdown, return, lookback, aggregation methods to Performance"
```

---

### Task 5: Update analytics .pyi stub

**Files:**
- Modify: `finstack-py/finstack/core/analytics/__init__.pyi`

**Step 1: Read the current stub**

Read: `finstack-py/finstack/core/analytics/__init__.pyi`

**Step 2: Add all new method signatures** to the `Performance` class in the stub.

Group methods with comments: `# Risk Metrics`, `# Rolling Metrics`, `# Benchmark-Relative`, `# Drawdown`, `# Return Helpers`, `# Lookback Selection`, `# Aggregation`, `# Consecutive`.

**Step 3: Verify stub syntax**

Run: `python -c "import ast; ast.parse(open('finstack-py/finstack/core/analytics/__init__.pyi').read()); print('OK')"`
Expected: `OK`

**Step 4: Commit**

```bash
git add finstack-py/finstack/core/analytics/__init__.pyi
git commit -m "feat(stubs): add all analytics metric methods to Performance .pyi stub"
```

---

### Task 6: Write analytics tests

**Files:**
- Create: `finstack-py/tests/core/test_analytics_performance.py`

**Step 1: Write test file**

```python
"""Tests for Performance facade analytics methods."""
import pytest

from finstack.core.analytics import Performance


# Use a fixture with known returns for deterministic testing
RETURNS = [0.01, -0.02, 0.015, 0.005, -0.01, 0.02, -0.005, 0.012, 0.008, -0.003]
BENCHMARK = [0.005, -0.01, 0.01, 0.003, -0.005, 0.015, -0.002, 0.008, 0.006, -0.001]


class TestRiskMetrics:
    """Test risk metric methods on Performance."""

    @pytest.fixture
    def perf(self) -> Performance:
        # Construct Performance from a Polars DataFrame or list of returns
        # Adapt constructor based on actual PyPerformance::new signature
        ...

    def test_sharpe_returns_float(self, perf: Performance) -> None:
        result = perf.sharpe()
        assert isinstance(result, float)

    def test_sortino_returns_float(self, perf: Performance) -> None:
        result = perf.sortino()
        assert isinstance(result, float)

    def test_volatility_positive(self, perf: Performance) -> None:
        result = perf.volatility()
        assert result > 0.0

    def test_value_at_risk_with_confidence(self, perf: Performance) -> None:
        var_95 = perf.value_at_risk(0.95)
        var_99 = perf.value_at_risk(0.99)
        assert isinstance(var_95, float)
        assert isinstance(var_99, float)

    def test_all_risk_metrics_callable(self, perf: Performance) -> None:
        """Smoke test: all risk metric methods should be callable."""
        methods = [
            'sharpe', 'sortino', 'volatility', 'cagr', 'calmar',
            'kurtosis', 'skewness', 'mean_return', 'geometric_mean',
            'gain_to_pain', 'tail_ratio', 'ulcer_index',
            'pain_index', 'pain_ratio', 'martin_ratio',
            'burke_ratio', 'sterling_ratio', 'recovery_factor',
        ]
        for method_name in methods:
            method = getattr(perf, method_name)
            result = method()
            assert isinstance(result, float), f"{method_name} did not return float"


class TestBenchmarkMetrics:
    """Test benchmark-relative methods."""

    @pytest.fixture
    def perf_with_bench(self) -> Performance:
        # Construct Performance with benchmark
        ...

    def test_beta_without_benchmark_raises(self, perf: Performance) -> None:
        with pytest.raises(ValueError, match="benchmark"):
            perf.beta()

    def test_beta_with_benchmark(self, perf_with_bench: Performance) -> None:
        result = perf_with_bench.beta()
        assert isinstance(result, float)


class TestDrawdownMetrics:
    """Test drawdown methods."""

    def test_max_drawdown_non_negative(self, perf: Performance) -> None:
        result = perf.max_drawdown()
        assert result >= 0.0

    def test_drawdown_series_length(self, perf: Performance) -> None:
        series = perf.drawdown_series()
        assert len(series) > 0
```

NOTE: Adapt the fixture to match the actual constructor signature of `PyPerformance`. It may take a Polars DataFrame, in which case you'll need `import polars as pl` and construct a DataFrame with date + price columns.

**Step 2: Run tests**

Run: `cd finstack-py && pytest tests/core/test_analytics_performance.py -v`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add finstack-py/tests/core/test_analytics_performance.py
git commit -m "test: add analytics Performance facade test suite"
```

---

## Stream 2: Market Data Missing Types (P0)

### Task 7: Add PriceCurve binding

**Files:**
- Read: `finstack/core/src/market_data/term_structures/price_curve.rs` (Rust source)
- Read: `finstack-py/src/core/market_data/term_structures.rs` (existing curve patterns)
- Modify: `finstack-py/src/core/market_data/term_structures.rs` (add PriceCurve)

**Step 1: Read the Rust PriceCurve type** to understand its API (constructor, methods, fields).

**Step 2: Add `PyPriceCurve`** following the exact pattern of `PyDiscountCurve`:

```rust
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "PriceCurve",
)]
pub struct PyPriceCurve {
    pub(crate) inner: Arc<PriceCurve>,
}

impl PyPriceCurve {
    pub(crate) fn new(inner: Arc<PriceCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPriceCurve {
    #[new]
    #[pyo3(signature = (id, base_date, knots, interp=None, extrapolation=None))]
    fn ctor(
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        interp: Option<Bound<'_, PyAny>>,
        extrapolation: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Follow DiscountCurve pattern exactly
        let bd = py_to_date(&base_date).context("base_date")?;
        let interp_style = interp.map(|i| parse_interp(&i)).transpose()?;
        let extrap = extrapolation.map(|e| parse_extrapolation(&e)).transpose()?;

        let curve = PriceCurve::builder()
            .id(CurveId::from(id))
            .base_date(bd)
            .knots(knots)
            // ... set interp and extrapolation if provided
            .build()
            .map_err(core_to_py)?;

        Ok(Self { inner: Arc::new(curve) })
    }

    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner.knots().to_vec()
    }

    #[pyo3(text_signature = "($self, t)")]
    fn price(&self, t: f64) -> PyResult<f64> {
        self.inner.price(t).map_err(core_to_py)
    }
}
```

NOTE: Read the actual `PriceCurve` builder API — it may differ from `DiscountCurve`. Adapt field names and methods accordingly.

**Step 3: Register in the term_structures `register()` function**

Add `module.add_class::<PyPriceCurve>()?;` and add `"PriceCurve"` to the exports vec.

**Step 4: Build and smoke test**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.core.market_data.term_structures import PriceCurve; print('OK')"`

**Step 5: Commit**

```bash
git add finstack-py/src/core/market_data/term_structures.rs
git commit -m "feat(py): add PriceCurve binding"
```

---

### Task 8: Add VolatilityIndexCurve binding

**Files:**
- Read: `finstack/core/src/market_data/term_structures/vol_index_curve.rs`
- Modify: `finstack-py/src/core/market_data/term_structures.rs`

**Step 1:** Follow the exact same pattern as Task 7 but for `VolatilityIndexCurve`.

Key method: `vol(t: float) -> float` instead of `price(t)`.

**Step 2: Register, build, test, commit**

```bash
git commit -m "feat(py): add VolatilityIndexCurve binding"
```

---

### Task 9: Add FlatCurve binding

**Files:**
- Read: `finstack/core/src/market_data/term_structures/flat.rs`
- Modify: `finstack-py/src/core/market_data/term_structures.rs`

**Step 1:** Add `PyFlatCurve` as a convenience wrapper:

```rust
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "FlatCurve",
)]
pub struct PyFlatCurve {
    pub(crate) inner: FlatCurve,
}

#[pymethods]
impl PyFlatCurve {
    #[new]
    #[pyo3(signature = (value, base_date, day_count, id))]
    fn ctor(
        value: f64,
        base_date: Bound<'_, PyAny>,
        day_count: Bound<'_, PyAny>,
        id: &str,
    ) -> PyResult<Self> {
        let bd = py_to_date(&base_date).context("base_date")?;
        let dc = parse_day_count(&day_count)?;
        let inner = FlatCurve::new(value, bd, dc, id);
        Ok(Self { inner })
    }

    fn as_discount_curve(&self) -> PyResult<PyDiscountCurve> {
        let dc = self.inner.as_discount_curve().map_err(core_to_py)?;
        Ok(PyDiscountCurve::new(Arc::new(dc)))
    }
}
```

NOTE: Check if `FlatCurve` has `as_discount_curve()` or similar conversion methods.

**Step 2: Register, build, test, commit**

```bash
git commit -m "feat(py): add FlatCurve convenience binding"
```

---

### Task 10: Add InflationIndex binding

**Files:**
- Read: `finstack/core/src/market_data/scalars/inflation_index.rs`
- Modify: `finstack-py/src/core/market_data/scalars.rs` (or create new file if scalars is a directory)

**Step 1:** Read the Rust `InflationIndex` API.

**Step 2:** Add `PyInflationIndex` following the scalars pattern:

```rust
#[pyclass(
    module = "finstack.core.market_data.scalars",
    name = "InflationIndex",
)]
pub struct PyInflationIndex {
    pub(crate) inner: Arc<InflationIndex>,
}

#[pymethods]
impl PyInflationIndex {
    // Constructor and methods matching Rust API
    fn at_date(&self, date: Bound<'_, PyAny>) -> PyResult<f64> { ... }
    fn growth_factor(&self, from: Bound<'_, PyAny>, to: Bound<'_, PyAny>) -> PyResult<f64> { ... }
}
```

**Step 3: Register, build, test, commit**

```bash
git commit -m "feat(py): add InflationIndex binding"
```

---

### Task 11: Add MarketContext.roll() and serialization

**Files:**
- Read: `finstack/core/src/market_data/context/ops_roll.rs`
- Read: `finstack/core/src/market_data/context/state_serde.rs`
- Modify: `finstack-py/src/core/market_data/context.rs`

**Step 1:** Add `roll` method to `PyMarketContext`:

```rust
#[pyo3(signature = (from_date, to_date, calendar=None))]
fn roll(
    &self,
    from_date: Bound<'_, PyAny>,
    to_date: Bound<'_, PyAny>,
    calendar: Option<&PyCalendar>,
) -> PyResult<Self> {
    let fd = py_to_date(&from_date).context("from_date")?;
    let td = py_to_date(&to_date).context("to_date")?;
    let cal = calendar.map(|c| &c.inner as &dyn HolidayCalendar);
    let rolled = self.inner.roll(fd, td, cal).map_err(core_to_py)?;
    Ok(Self { inner: rolled })
}
```

**Step 2:** Add insert/get methods for new curve types:

```rust
fn insert_price_curve(&mut self, curve: &PyPriceCurve) {
    self.inner.insert_price_curve((*curve.inner).clone());
}

fn get_price_curve(&self, id: &str) -> PyResult<Option<PyPriceCurve>> {
    match self.inner.get_price_curve(id) {
        Ok(c) => Ok(Some(PyPriceCurve::new(c))),
        Err(_) => Ok(None),
    }
}

// Repeat for vol_index, inflation_index
```

**Step 3:** Add serialization:

```rust
fn to_state(&self, py: Python<'_>) -> PyResult<String> {
    let state = MarketContextState::from(&self.inner);
    serde_json::to_string(&state)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[classmethod]
fn from_state(_cls: &Bound<'_, PyType>, state: &str) -> PyResult<Self> {
    let parsed: MarketContextState = serde_json::from_str(state)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let ctx = MarketContext::from(parsed).map_err(core_to_py)?;
    Ok(Self { inner: ctx })
}
```

NOTE: Verify the exact serialization API — it may use `MarketContextState` or a different type.

**Step 4: Build and test**

Run: `cd finstack-py && maturin develop --release`

**Step 5: Commit**

```bash
git add finstack-py/src/core/market_data/context.rs
git commit -m "feat(py): add MarketContext.roll(), serialization, and new curve type accessors"
```

---

### Task 12: Update market data .pyi stubs

**Files:**
- Modify: `finstack-py/finstack/core/market_data/term_structures.pyi` (add PriceCurve, VolatilityIndexCurve, FlatCurve)
- Modify: `finstack-py/finstack/core/market_data/scalars.pyi` (add InflationIndex)
- Modify: `finstack-py/finstack/core/market_data/__init__.pyi` (add new re-exports)
- Modify: `finstack-py/finstack/core/market_data/context.pyi` (add roll, serialization, new accessors)

**Step 1:** Add stub entries for all new types and methods.

**Step 2:** Verify stub syntax.

Run: `python -c "import ast; ast.parse(open('finstack-py/finstack/core/market_data/term_structures.pyi').read()); print('OK')"`

**Step 3: Commit**

```bash
git add finstack-py/finstack/core/market_data/*.pyi
git commit -m "feat(stubs): add PriceCurve, VolatilityIndexCurve, FlatCurve, InflationIndex, MarketContext.roll() stubs"
```

---

## Stream 3: Math Advanced Types (P1)

### Task 13: Add Compounding binding

**Files:**
- Read: `finstack/core/src/math/compounding.rs`
- Create: `finstack-py/src/core/math/compounding.rs`
- Modify: `finstack-py/src/core/math/mod.rs`

**Step 1:** Read the Rust `Compounding` struct API.

**Step 2:** Create `finstack-py/src/core/math/compounding.rs`:

```rust
use finstack_core::math::compounding::Compounding;
use pyo3::prelude::*;
use crate::errors::core_to_py;

#[pyclass(
    name = "Compounding",
    module = "finstack.core.math.compounding",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyCompounding {
    pub(crate) inner: Compounding,
}

impl PyCompounding {
    pub(crate) const fn new(inner: Compounding) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCompounding {
    #[classattr]
    const ANNUAL: Self = Self::new(Compounding::Annual);
    #[classattr]
    const SEMI_ANNUAL: Self = Self::new(Compounding::SemiAnnual);
    #[classattr]
    const QUARTERLY: Self = Self::new(Compounding::Quarterly);
    #[classattr]
    const MONTHLY: Self = Self::new(Compounding::Monthly);
    #[classattr]
    const CONTINUOUS: Self = Self::new(Compounding::Continuous);

    #[pyo3(text_signature = "($self, rate, time)")]
    fn compound_factor(&self, rate: f64, time: f64) -> PyResult<f64> {
        self.inner.compound_factor(rate, time).map_err(core_to_py)
    }

    #[pyo3(text_signature = "($self, rate, time)")]
    fn discount_factor(&self, rate: f64, time: f64) -> PyResult<f64> {
        self.inner.discount_factor(rate, time).map_err(core_to_py)
    }

    #[staticmethod]
    #[pyo3(text_signature = "(from_compounding, to_compounding, rate)")]
    fn equivalent_rate(
        from: &PyCompounding,
        to: &PyCompounding,
        rate: f64,
    ) -> PyResult<f64> {
        Compounding::equivalent_rate(from.inner, to.inner, rate)
            .map_err(core_to_py)
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "compounding")?;
    module.add_class::<PyCompounding>()?;
    let exports = ["Compounding"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
```

NOTE: Verify actual `Compounding` variant names and method signatures against the Rust source.

**Step 3:** Register in `finstack-py/src/core/math/mod.rs`:

Add `pub(crate) mod compounding;` and call `compounding::register(py, &module)?` in the register function.

**Step 4: Build, test, commit**

```bash
git commit -m "feat(py): add Compounding binding with frequency conversion"
```

---

### Task 14: Add TimeGrid binding

**Files:**
- Read: `finstack/core/src/math/time_grid.rs`
- Create: `finstack-py/src/core/math/time_grid.rs`
- Modify: `finstack-py/src/core/math/mod.rs`

Follow the same pattern as Task 13 but for `TimeGrid`. Key methods:

| Method | Signature |
|---|---|
| `__init__` (from times) | `(times: list[float])` |
| `uniform` (classmethod) | `(start: float, end: float, n_steps: int) -> TimeGrid` |
| `times` (property) | `-> list[float]` |
| `dt` (property) | `-> list[float]` |
| `map_date_to_step` | `(t: float) -> int` |
| `map_dates_to_steps` | `(times: list[float]) -> list[int]` |

```bash
git commit -m "feat(py): add TimeGrid binding for time discretization"
```

---

### Task 15: Add SobolRng binding

**Files:**
- Read: `finstack/core/src/math/random/sobol.rs`
- Modify: `finstack-py/src/core/math/random.rs`

**Step 1:** Add `PySobolRng` to the existing random module:

```rust
#[pyclass(name = "SobolRng", module = "finstack.core.math.random")]
pub struct PySobolRng {
    inner: SobolRng,
}

#[pymethods]
impl PySobolRng {
    #[new]
    #[pyo3(signature = (dimension, seed=0))]
    fn new(dimension: usize, seed: Option<u64>) -> PyResult<Self> {
        let s = seed.unwrap_or(0);
        let inner = SobolRng::new(dimension, s)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn next(&mut self) -> Vec<f64> {
        self.inner.next()
    }

    fn skip(&mut self, n: usize) {
        self.inner.skip(n);
    }
}
```

Also add `MAX_SOBOL_DIMENSION` as a module constant.

**Step 2:** Register `PySobolRng` in the random module's register function.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add SobolRng quasi-random sequence generator"
```

---

### Task 16: Add moment_match() to stats

**Files:**
- Modify: `finstack-py/src/core/math/stats.rs`

**Step 1:** Add `moment_match` pyfunction:

```rust
#[pyfunction(name = "moment_match")]
#[pyo3(text_signature = "(target_mean, target_variance, samples)")]
fn moment_match_py(
    target_mean: f64,
    target_variance: f64,
    samples: Vec<f64>,
) -> PyResult<Vec<f64>> {
    finstack_core::math::stats::moment_match(target_mean, target_variance, &samples)
        .map_err(core_to_py)
}
```

NOTE: Verify the exact signature — `moment_match` may return a `Vec<f64>` or `Result<Vec<f64>>`.

**Step 2:** Add to exports and register.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add moment_match() to math.stats"
```

---

### Task 17: Update math .pyi stubs

**Files:**
- Create: `finstack-py/finstack/core/math/compounding.pyi`
- Create: `finstack-py/finstack/core/math/time_grid.pyi`
- Modify: `finstack-py/finstack/core/math/random.pyi` (add SobolRng, MAX_SOBOL_DIMENSION)
- Modify: `finstack-py/finstack/core/math/stats.pyi` (add moment_match)
- Modify: `finstack-py/finstack/core/math/__init__.pyi` (add new module imports)

**Step 1:** Write stubs for all new types.

**Step 2:** Verify syntax.

**Step 3: Commit**

```bash
git add finstack-py/finstack/core/math/*.pyi
git commit -m "feat(stubs): add Compounding, TimeGrid, SobolRng, moment_match stubs"
```

---

## Stream 4: FX Providers + Type IDs (P1)

### Task 18: Add missing ID types

**Files:**
- Modify: `finstack-py/src/core/types.rs`
- Modify: `finstack-py/finstack/core/types.pyi`

**Step 1:** Add three new ID types using the existing `declare_id_type!` macro:

```rust
declare_id_type! {
    /// Type-safe identifier for holiday calendars.
    "CalendarId", PyCalendarId, CalendarId
}
declare_id_type! {
    /// Type-safe identifier for securitized pools.
    "PoolId", PyPoolId, PoolId
}
declare_id_type! {
    /// Type-safe identifier for structured deals.
    "DealId", PyDealId, DealId
}
```

**Step 2:** Register in the `register()` function:

```rust
module.add_class::<PyCalendarId>()?;
module.add_class::<PyPoolId>()?;
module.add_class::<PyDealId>()?;
```

Add `"CalendarId"`, `"PoolId"`, `"DealId"` to the exports.

**Step 3:** Update `finstack-py/finstack/core/types.pyi` — add stub entries following the same pattern as `CurveId`:

```python
class CalendarId:
    def __init__(self, id: str) -> None: ...
    def as_str(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Repeat for PoolId, DealId
```

**Step 4: Build, test, commit**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.core.types import CalendarId, PoolId, DealId; print(CalendarId('US').as_str())"`
Expected: `US`

```bash
git add finstack-py/src/core/types.rs finstack-py/finstack/core/types.pyi
git commit -m "feat(py): add CalendarId, PoolId, DealId type aliases"
```

---

### Task 19: Add FxQuery binding

**Files:**
- Modify: `finstack-py/src/core/market_data/fx.rs`
- Modify: `finstack-py/finstack/core/market_data/fx.pyi`

**Step 1:** Read `finstack/core/src/money/fx/mod.rs` for `FxQuery` definition.

**Step 2:** Add `PyFxQuery`:

```rust
#[pyclass(
    name = "FxQuery",
    module = "finstack.core.market_data.fx",
    frozen,
    from_py_object,
)]
#[derive(Clone, Debug)]
pub struct PyFxQuery {
    pub(crate) inner: FxQuery,
}

#[pymethods]
impl PyFxQuery {
    #[new]
    #[pyo3(signature = (from_ccy, to_ccy, on, policy=None))]
    fn ctor(
        from_ccy: Bound<'_, PyAny>,
        to_ccy: Bound<'_, PyAny>,
        on: Bound<'_, PyAny>,
        policy: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let from = extract_currency(&from_ccy)?;
        let to = extract_currency(&to_ccy)?;
        let date = py_to_date(&on).context("on")?;
        let pol = policy.map(|p| parse_fx_policy(&p)).transpose()?;

        let inner = match pol {
            Some(p) => FxQuery::with_policy(from, to, date, p),
            None => FxQuery::new(from, to, date),
        };
        Ok(Self { inner })
    }

    #[getter]
    fn from_currency(&self) -> PyCurrency { ... }
    #[getter]
    fn to_currency(&self) -> PyCurrency { ... }
    #[getter]
    fn on(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { ... }
    #[getter]
    fn policy(&self) -> PyFxConversionPolicy { ... }
}
```

**Step 3:** Register and update stub.

**Step 4: Commit**

```bash
git commit -m "feat(py): add FxQuery binding"
```

---

### Task 20: Add SimpleFxProvider and BumpedFxProvider

**Files:**
- Read: `finstack/core/src/money/fx/providers.rs`
- Modify: `finstack-py/src/core/market_data/fx.rs`
- Modify: `finstack-py/finstack/core/market_data/fx.pyi`

**Step 1:** Add `PySimpleFxProvider`:

```rust
#[pyclass(name = "SimpleFxProvider", module = "finstack.core.market_data.fx")]
pub struct PySimpleFxProvider {
    inner: SimpleFxProvider,
}

#[pymethods]
impl PySimpleFxProvider {
    #[new]
    fn new() -> Self {
        Self { inner: SimpleFxProvider::new() }
    }

    #[pyo3(signature = (from_ccy, to_ccy, rate))]
    fn set_quote(
        &mut self,
        from_ccy: Bound<'_, PyAny>,
        to_ccy: Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<()> {
        let from = extract_currency(&from_ccy)?;
        let to = extract_currency(&to_ccy)?;
        self.inner.set_quote(from, to, rate);
        Ok(())
    }

    fn to_matrix(&self) -> PyResult<PyFxMatrix> {
        let matrix = FxMatrix::new(Arc::new(self.inner.clone()));
        Ok(PyFxMatrix::new(matrix))
    }
}
```

**Step 2:** Add `PyBumpedFxProvider` following the Rust API.

NOTE: `BumpedFxProvider` takes an `Arc<dyn FxProvider>` — you may need to handle this via a wrapper that accepts either `FxMatrix` or `SimpleFxProvider`.

**Step 3: Register, build, test, commit**

```bash
git commit -m "feat(py): add SimpleFxProvider and BumpedFxProvider bindings"
```

---

## Stream 5: Dates Utilities (P2)

### Task 21: Add missing IMM functions

**Files:**
- Modify: `finstack-py/src/core/dates/imm.rs`
- Modify: `finstack-py/finstack/core/dates/imm.pyi`

**Step 1:** Add the following functions following the existing pattern:

```rust
#[pyfunction(name = "is_imm_date", text_signature = "(date)")]
fn is_imm_date_py(date: Bound<'_, PyAny>) -> PyResult<bool> {
    let d = py_to_date(&date).context("date")?;
    Ok(is_imm_date(d))
}

#[pyfunction(name = "third_wednesday", text_signature = "(date)")]
fn third_wednesday_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, third_wednesday(d))
}

#[pyfunction(name = "third_friday", text_signature = "(date)")]
fn third_friday_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, third_friday(d))
}

#[pyfunction(name = "sifma_settlement_date", text_signature = "(date)")]
fn sifma_settlement_date_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, sifma_settlement_date(d))
}

#[pyfunction(name = "next_sifma_settlement", text_signature = "(date)")]
fn next_sifma_settlement_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_sifma_settlement(d))
}

#[pyfunction(name = "next_equity_option_expiry", text_signature = "(date)")]
fn next_equity_option_expiry_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_equity_option_expiry(d))
}
```

**Step 2:** Register all functions in the `register()` function with `wrap_pyfunction!`.

**Step 3:** Update `imm.pyi` stub.

**Step 4: Build, test, commit**

```bash
git commit -m "feat(py): add is_imm_date, third_wednesday, third_friday, sifma functions"
```

---

### Task 22: Add date constants and HalfMonthModifiedFollowing

**Files:**
- Modify: `finstack-py/src/core/dates/mod.rs` (add constants)
- Modify: `finstack-py/src/core/dates/calendar.rs` (add HalfMonthModifiedFollowing)
- Modify: `finstack-py/finstack/core/dates/__init__.pyi`
- Modify: `finstack-py/finstack/core/dates/calendar.pyi`

**Step 1:** Add constants to dates module registration:

```rust
module.setattr("CALENDAR_DAYS_PER_YEAR", finstack_core::dates::CALENDAR_DAYS_PER_YEAR)?;
module.setattr("AVERAGE_DAYS_PER_YEAR", finstack_core::dates::AVERAGE_DAYS_PER_YEAR)?;
```

**Step 2:** Add `HalfMonthModifiedFollowing` classattr to `PyBusinessDayConvention`:

```rust
#[classattr]
const HALF_MONTH_MODIFIED_FOLLOWING: Self = Self {
    inner: BusinessDayConvention::HalfMonthModifiedFollowing,
};
```

Also update the `label()` match arm:

```rust
BusinessDayConvention::HalfMonthModifiedFollowing => "half_month_modified_following",
```

And `from_name()`:

```rust
"half_month_modified_following" | "halfmonthmodifiedfollowing" => {
    Ok(Self::new(BusinessDayConvention::HalfMonthModifiedFollowing))
}
```

**Step 3:** Update stubs.

**Step 4: Build, test, commit**

```bash
git commit -m "feat(py): add CALENDAR_DAYS_PER_YEAR, AVERAGE_DAYS_PER_YEAR, HalfMonthModifiedFollowing"
```

---

### Task 23: Add ScheduleWarning

**Files:**
- Modify: `finstack-py/src/core/dates/schedule.rs`
- Modify: `finstack-py/finstack/core/dates/schedule.pyi`

**Step 1:** Read `finstack/core/src/dates/schedule.rs` for the `ScheduleWarning` enum.

**Step 2:** Add `PyScheduleWarning` and update `PySchedule` to expose warnings:

```rust
#[pyclass(name = "ScheduleWarning", module = "finstack.core.dates", frozen)]
pub struct PyScheduleWarning {
    inner: ScheduleWarning,
}

#[pymethods]
impl PyScheduleWarning {
    #[getter]
    fn kind(&self) -> &str {
        // Match on variant
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ScheduleWarning('{}')", self.message())
    }
}
```

Update `PySchedule` to add a `warnings` getter.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add ScheduleWarning and Schedule.warnings property"
```

---

## Stream 6: Cashflow Minor (P2)

### Task 24: Add CashFlow builder methods and npv_amounts

**Files:**
- Modify: `finstack-py/src/core/cashflow/primitives.rs`
- Modify: `finstack-py/src/core/cashflow/discounting.rs` (or wherever NPV functions live)
- Modify: `finstack-py/finstack/core/cashflow/primitives.pyi`
- Modify: `finstack-py/finstack/core/cashflow/__init__.pyi`

**Step 1:** Add `with_rate` and `with_reset_date` to `PyCashFlow`:

```rust
#[pyo3(text_signature = "($self, rate)")]
fn with_rate(&self, rate: f64) -> Self {
    Self {
        inner: self.inner.clone().with_rate(rate),
    }
}

#[pyo3(text_signature = "($self, date)")]
fn with_reset_date(&self, date: Bound<'_, PyAny>) -> PyResult<Self> {
    let d = py_to_date(&date).context("date")?;
    Ok(Self {
        inner: self.inner.clone().with_reset_date(d),
    })
}
```

NOTE: Verify `CashFlow` has these methods. If `CashFlow` uses `clone()` + mutation or a builder pattern, adapt accordingly.

**Step 2:** Add `npv_amounts` pyfunction:

```rust
#[pyfunction(name = "npv_amounts")]
#[pyo3(signature = (cashflows, rate, base_date=None, day_count=None))]
fn npv_amounts_py(
    cashflows: Vec<(Bound<'_, PyAny>, f64)>,
    rate: f64,
    base_date: Option<Bound<'_, PyAny>>,
    day_count: Option<Bound<'_, PyAny>>,
) -> PyResult<f64> {
    let cfs: Vec<(Date, f64)> = cashflows.iter()
        .map(|(d, a)| Ok((py_to_date(d)?, *a)))
        .collect::<PyResult<_>>()?;
    let bd = base_date.map(|b| py_to_date(&b)).transpose()?;
    let dc = day_count.map(|d| parse_day_count(&d)).transpose()?;
    cashflow::npv_amounts(&cfs, rate, bd, dc)
        .map_err(core_to_py)
}
```

**Step 3:** Register `npv_amounts` in the cashflow module.

**Step 4:** Update stubs.

**Step 5: Build, test, commit**

```bash
git add finstack-py/src/core/cashflow/*.rs finstack-py/finstack/core/cashflow/*.pyi
git commit -m "feat(py): add CashFlow.with_rate(), with_reset_date(), and npv_amounts()"
```

---

## Final Verification

### Task 25: Run full verification suite

**Step 1: Build**

```bash
cd finstack-py && maturin develop --release
```

**Step 2: Run parity audit** (from design doc verification section)

```bash
python -c "
from finstack.core.analytics import Performance
from finstack.core.market_data.term_structures import (
    DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
    BaseCorrelationCurve, PriceCurve, VolatilityIndexCurve, FlatCurve,
)
from finstack.core.market_data.scalars import InflationIndex
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxQuery, SimpleFxProvider
from finstack.core.math.compounding import Compounding
from finstack.core.math.time_grid import TimeGrid
from finstack.core.math.random import SobolRng
from finstack.core.math.stats import moment_match
from finstack.core.types import CalendarId, PoolId, DealId
from finstack.core.dates import CALENDAR_DAYS_PER_YEAR, AVERAGE_DAYS_PER_YEAR
from finstack.core.dates.imm import is_imm_date, third_wednesday, third_friday
from finstack.core.cashflow import npv_amounts
print('PASS: All core Python binding parity imports successful')
"
```

**Step 3: Run test suite**

```bash
cd finstack-py && pytest tests/ -v --tb=short
```

**Step 4: Validate stubs**

```bash
find finstack-py/finstack -name '*.pyi' -exec python -c "import ast; ast.parse(open('{}').read())" \;
echo "All stubs valid"
```

**Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final parity verification fixes"
```
