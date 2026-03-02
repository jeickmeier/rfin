# Core Python Binding: 100% Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close every remaining gap between the Rust `finstack-core` public API and the `finstack.core` Python bindings, achieving 100% parity.

**Architecture:** Six independent work streams: (1) Analytics pandas DataFrame support, (2) Market Data missing curve types, (3) Math advanced types, (4) FX providers + Type IDs, (5) Dates utilities, (6) Cashflow minor methods. Streams can run in parallel since they touch non-overlapping files.

**NOTE:** The Performance class already has complete method coverage (~50 methods, 1069 lines in `performance.rs`). Stream 1 only needs to add pandas DataFrame input support — all analytics methods are already bound.

**Tech Stack:** Rust (pyo3 0.28), Python (.pyi type stubs), maturin build system

**Design Doc:** `docs/plans/2026-03-01-core-python-parity-design.md`

---

## Stream 1: Analytics — Pandas DataFrame Input Support (P0)

> **NOTE:** The Performance class already has complete method coverage (~50 methods in 1069 lines).
> All risk metrics, rolling metrics, benchmark-relative metrics, drawdown metrics, lookback returns,
> period stats, etc. are already bound. The only gap is that the constructor only accepts Polars
> DataFrames. This stream adds pandas DataFrame support.

### Task 1: Add pandas DataFrame input support to Performance constructor

**Files:**
- Modify: `finstack-py/src/core/analytics/performance.rs`

**Step 1: Read the existing Performance binding** to understand the current constructor.

Read: `finstack-py/src/core/analytics/performance.rs` (full file, 1069 lines)

Key observations:
- Constructor at line 208: `fn new(prices: PyDataFrame, ...)` — accepts only `pyo3_polars::PyDataFrame`
- Helper `extract_dates_and_prices(df: &DataFrame)` at line 27 — extracts dates from first column, prices from remaining columns
- The constructor passes extracted data to `finstack_core::analytics::Performance::new()`

**Step 2: Modify the constructor to accept `&Bound<'_, PyAny>` instead of `PyDataFrame`**

Change the constructor signature and add a conversion function:

```rust
/// Convert a Python object (pandas or polars DataFrame) to a Polars DataFrame.
///
/// For pandas DataFrames:
///   - Resets the index (so date index becomes a column)
///   - Converts to Polars via `polars.from_pandas()`
///
/// For Polars DataFrames:
///   - Extracts the inner `PyDataFrame` directly
fn py_to_polars_df(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<DataFrame> {
    // Try extracting as PyDataFrame first (Polars path)
    if let Ok(pdf) = obj.extract::<PyDataFrame>() {
        return Ok(pdf.0);
    }

    // Check if it's a pandas DataFrame by looking for the `reset_index` method
    if obj.hasattr("reset_index")? {
        // Import polars and convert: polars.from_pandas(df.reset_index())
        let pl = py.import("polars")?;
        let reset = obj.call_method0("reset_index")?;
        let polars_df = pl.call_method1("from_pandas", (&reset,))?;
        let pdf: PyDataFrame = polars_df.extract()?;
        return Ok(pdf.0);
    }

    Err(PyTypeError::new_err(
        "Expected a polars.DataFrame or pandas.DataFrame, got {}"
            .replace("{}", obj.get_type().name()?.to_str()?),
    ))
}
```

Then update the constructor:

```rust
#[new]
#[pyo3(signature = (prices, benchmark_ticker=None, freq="daily", log_returns=false))]
fn new(
    py: Python<'_>,
    prices: &Bound<'_, PyAny>,
    benchmark_ticker: Option<&str>,
    freq: &str,
    log_returns: bool,
) -> PyResult<Self> {
    let period_kind = parse_freq(freq)?;
    let df = py_to_polars_df(py, prices)?;
    let (dates, price_cols, tickers) = extract_dates_and_prices(&df)?;
    let inner = Performance::new(
        dates,
        price_cols,
        tickers,
        benchmark_ticker,
        period_kind,
        log_returns,
    )
    .map_err(core_to_py)?;
    Ok(Self { inner })
}
```

**Step 3: Update the module docstring** at the top of the file to mention pandas support:

Change line 3 from:

```
//! Accepts Polars DataFrames on the Python side, extracts columns to Rust
```

to:

```
//! Accepts Polars or pandas DataFrames on the Python side, extracts columns to Rust
```

Also update the class docstring (lines 166-181) to mention pandas DataFrames:

```rust
/// Performance analytics engine.
///
/// Construct from a Polars or pandas DataFrame. For Polars, the first column
/// must be a Date column followed by price columns (one per ticker). For pandas,
/// the index should contain dates and each column should be a price series.
///
/// Parameters
/// ----------
/// prices : polars.DataFrame | pandas.DataFrame
///     For polars: first column is Date, remaining are price series.
///     For pandas: index is dates, columns are price series.
/// benchmark_ticker : str, optional
///     Name of the benchmark column. Defaults to the first price column.
/// freq : str
///     Observation frequency: ``"daily"``, ``"weekly"``, ``"monthly"``,
///     ``"quarterly"``, ``"semiannual"``, ``"annual"``.
/// log_returns : bool
///     If True, use log returns; otherwise use simple returns.
```

**Step 4: Build and smoke test**

Run: `cd finstack-py && maturin develop --release`

Then run the following smoke test:

```bash
python -c "
import pandas as pd, polars as pl
from finstack.core.analytics import Performance
df_pl = pl.DataFrame({'date': pl.date_range(pl.date(2024,1,1), pl.date(2024,6,30), eager=True), 'SPY': [100+i*0.5 for i in range(182)]})
p1 = Performance(df_pl, freq='daily')
print('Polars:', p1.sharpe())
import datetime
dates = pd.date_range('2024-01-01', periods=182, freq='D')
df_pd = pd.DataFrame({'SPY': [100+i*0.5 for i in range(182)]}, index=dates)
p2 = Performance(df_pd, freq='daily')
print('Pandas:', p2.sharpe())
"
```

Expected: Both print Sharpe ratios (should be similar values).

**Step 5: Commit**

```bash
git add finstack-py/src/core/analytics/performance.rs
git commit -m "feat(py): accept pandas or polars DataFrame in Performance constructor"
```

---

### Task 2: Update analytics .pyi stub for pandas input

**Files:**
- Modify: `finstack-py/finstack/core/analytics/__init__.pyi`

**Step 1: Read the current stub**

Read: `finstack-py/finstack/core/analytics/__init__.pyi`

**Step 2: Update the `Performance.__init__` type signature**

Change the `prices` parameter type from `polars.DataFrame` to `polars.DataFrame | pandas.DataFrame`:

```python
from __future__ import annotations
import polars
import pandas

class Performance:
    def __init__(
        self,
        prices: polars.DataFrame | pandas.DataFrame,
        benchmark_ticker: str | None = None,
        freq: str = "daily",
        log_returns: bool = False,
    ) -> None: ...
```

NOTE: Add `import pandas` at the top of the stub if not already present.

**Step 3: Verify stub syntax**

Run: `python -c "import ast; ast.parse(open('finstack-py/finstack/core/analytics/__init__.pyi').read()); print('OK')"`
Expected: `OK`

**Step 4: Commit**

```bash
git add finstack-py/finstack/core/analytics/__init__.pyi
git commit -m "feat(stubs): update Performance stub to accept pandas DataFrame"
```

---

### Task 3: Write pandas input tests

**Files:**
- Create: `finstack-py/tests/core/test_analytics_pandas.py`

**Step 1: Write test file**

```python
"""Tests for Performance pandas DataFrame input support."""
import datetime

import pandas as pd
import polars as pl
import pytest

from finstack.core.analytics import Performance


@pytest.fixture
def sample_dates():
    return pd.date_range("2024-01-02", periods=100, freq="B")


@pytest.fixture
def sample_prices(sample_dates):
    import numpy as np
    np.random.seed(42)
    n = len(sample_dates)
    return pd.DataFrame(
        {
            "AAPL": 150.0 * (1 + np.random.randn(n) * 0.02).cumprod(),
            "MSFT": 300.0 * (1 + np.random.randn(n) * 0.015).cumprod(),
        },
        index=sample_dates,
    )


class TestPandasInput:
    """Test that Performance accepts pandas DataFrames."""

    def test_pandas_constructor(self, sample_prices):
        perf = Performance(sample_prices, freq="daily")
        assert perf.ticker_names == ["AAPL", "MSFT"]

    def test_pandas_sharpe(self, sample_prices):
        perf = Performance(sample_prices, freq="daily")
        result = perf.sharpe()
        # Should return a DataFrame with ticker and sharpe columns
        assert result is not None

    def test_pandas_matches_polars(self, sample_prices):
        """Pandas and polars input should produce identical results."""
        perf_pd = Performance(sample_prices, freq="daily")

        # Convert to polars equivalent
        df_pl = pl.from_pandas(sample_prices.reset_index())
        perf_pl = Performance(df_pl, freq="daily")

        # Compare a scalar metric
        pd_sharpe = perf_pd.sharpe().to_pandas()
        pl_sharpe = perf_pl.sharpe().to_pandas()
        pd.testing.assert_frame_equal(pd_sharpe, pl_sharpe, atol=1e-10)

    def test_pandas_with_benchmark(self, sample_prices):
        perf = Performance(sample_prices, benchmark_ticker="MSFT", freq="daily")
        assert perf.benchmark_idx == 1

    def test_invalid_input_raises(self):
        with pytest.raises(TypeError):
            Performance("not a dataframe", freq="daily")

    def test_polars_still_works(self, sample_prices):
        """Existing polars input should continue to work."""
        df_pl = pl.from_pandas(sample_prices.reset_index())
        perf = Performance(df_pl, freq="daily")
        assert perf.ticker_names == ["AAPL", "MSFT"]
```

**Step 2: Run tests**

Run: `cd finstack-py && pytest tests/core/test_analytics_pandas.py -v`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add finstack-py/tests/core/test_analytics_pandas.py
git commit -m "test: add pandas DataFrame input tests for Performance"
```

---

## Stream 2: Market Data Missing Types (P0)

### Task 4: Add PriceCurve binding

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

### Task 5: Add VolatilityIndexCurve binding

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

### Task 6: Add FlatCurve binding

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

### Task 7: Add InflationIndex binding

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

### Task 8: Add MarketContext.roll() and serialization

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

### Task 9: Update market data .pyi stubs

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

### Task 10: Add Compounding binding

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

### Task 11: Add TimeGrid binding

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

### Task 12: Add SobolRng binding

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

### Task 13: Add moment_match() to stats

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

### Task 14: Update math .pyi stubs

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

### Task 15: Add missing ID types

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

### Task 16: Add FxQuery binding

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

### Task 17: Add SimpleFxProvider and BumpedFxProvider

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

### Task 18: Add missing IMM functions

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

### Task 19: Add date constants and HalfMonthModifiedFollowing

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

### Task 20: Add ScheduleWarning

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

### Task 21: Add CashFlow builder methods and npv_amounts

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

### Task 22: Run full verification suite

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
