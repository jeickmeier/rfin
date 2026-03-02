# Statements Python Bindings Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Achieve 100% Python binding parity with the Rust `finstack_statements` public API for core + analysis modules.

**Architecture:** PyO3 wrapper structs with `inner` field pattern, `stmt_to_py` error conversion, `py.detach()` for compute. New analysis bindings in `finstack-py/src/statements/analysis/` with registration via `register()` functions.

**Tech Stack:** Rust (PyO3), Python (.pyi stubs), pytest

---

## Phase 1: Core Type Completeness

### Task 1: Add `is_amount` and `as_money()` to AmountOrScalar

**Files:**
- Modify: `finstack-py/src/statements/types/value.rs`
- Modify: `finstack-py/finstack/statements/types/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

Add to `finstack-py/tests/test_statements_parity.py`:

```python
def test_amount_or_scalar_is_amount():
    """Test is_amount property on AmountOrScalar."""
    from finstack.core.currency import USD
    from finstack.statements.types import AmountOrScalar

    scalar = AmountOrScalar.scalar(42.0)
    assert scalar.is_scalar is True
    assert scalar.is_amount is False

    amount = AmountOrScalar.amount(100.0, USD)
    assert amount.is_scalar is False
    assert amount.is_amount is True


def test_amount_or_scalar_as_money():
    """Test as_money method on AmountOrScalar."""
    from finstack.core.currency import USD
    from finstack.core.money import Money
    from finstack.statements.types import AmountOrScalar

    scalar = AmountOrScalar.scalar(42.0)
    assert scalar.as_money() is None

    amount = AmountOrScalar.amount(100.0, USD)
    money = amount.as_money()
    assert money is not None
    assert money.amount == pytest.approx(100.0)
```

**Step 2: Run test to verify it fails**

Run: `cd finstack-py && maturin develop --release && pytest tests/test_statements_parity.py::test_amount_or_scalar_is_amount -v`
Expected: AttributeError: 'AmountOrScalar' object has no attribute 'is_amount'

**Step 3: Implement in Rust**

In `finstack-py/src/statements/types/value.rs`, add inside the `#[pymethods] impl PyAmountOrScalar` block, after the `is_scalar` getter:

```rust
    #[getter]
    /// Check if this is a currency amount.
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if amount, False if scalar
    fn is_amount(&self) -> bool {
        matches!(self.inner, AmountOrScalar::Amount(_))
    }

    /// Get the value as a Money object.
    ///
    /// Returns
    /// -------
    /// Money | None
    ///     Money if this is an amount, None if scalar
    fn as_money(&self) -> Option<crate::core::money::PyMoney> {
        match &self.inner {
            AmountOrScalar::Amount(m) => Some(crate::core::money::PyMoney::new(*m)),
            AmountOrScalar::Scalar(_) => None,
        }
    }
```

**Step 4: Update .pyi stub**

In `finstack-py/finstack/statements/types/__init__.pyi`, add to `AmountOrScalar` class:

```python
    @property
    def is_amount(self) -> bool: ...

    def as_money(self) -> Money | None: ...
```

And add `Money` to the imports at the top.

**Step 5: Run tests to verify they pass**

Run: `cd finstack-py && maturin develop --release && pytest tests/test_statements_parity.py::test_amount_or_scalar_is_amount tests/test_statements_parity.py::test_amount_or_scalar_as_money -v`
Expected: PASS

**Step 6: Commit**

```bash
git add finstack-py/src/statements/types/value.rs finstack-py/finstack/statements/types/__init__.pyi finstack-py/tests/test_statements_parity.py
git commit -m "feat(py): add is_amount and as_money to AmountOrScalar"
```

---

### Task 2: Add `get_money()` and `get_scalar()` to StatementResult

**Files:**
- Modify: `finstack-py/src/statements/evaluator/mod.rs`
- Modify: `finstack-py/finstack/statements/evaluator/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_statement_result_get_money():
    """Test get_money method on StatementResult."""
    from finstack.core.currency import USD
    from finstack.core.dates import PeriodId
    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("money_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_money(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.amount(100000.0, USD)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.amount(110000.0, USD)),
        ],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    q1 = PeriodId.quarter(2025, 1)
    money = results.get_money("revenue", q1)
    assert money is not None
    assert money.amount == pytest.approx(100000.0)

    # Scalar node should return None for get_money
    builder2 = ModelBuilder.new("scalar_test")
    builder2.periods("2025Q1..Q2", None)
    builder2.value_scalar("ratio", [(PeriodId.quarter(2025, 1), 0.5)])
    model2 = builder2.build()
    results2 = evaluator.evaluate(model2)
    assert results2.get_money("ratio", q1) is None


def test_statement_result_get_scalar():
    """Test get_scalar method on StatementResult."""
    from finstack.core.dates import PeriodId
    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("scalar_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_scalar(
        "margin",
        [(PeriodId.quarter(2025, 1), 0.35)],
    )
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    q1 = PeriodId.quarter(2025, 1)
    scalar = results.get_scalar("margin", q1)
    assert scalar is not None
    assert scalar == pytest.approx(0.35)
```

**Step 2: Run test to verify it fails**

Run: `cd finstack-py && maturin develop --release && pytest tests/test_statements_parity.py::test_statement_result_get_money -v`
Expected: AttributeError: 'StatementResult' object has no attribute 'get_money'

**Step 3: Implement in Rust**

In `finstack-py/src/statements/evaluator/mod.rs`, add inside the `#[pymethods] impl PyStatementResult` block:

```rust
    #[pyo3(text_signature = "(self, node_id, period_id)")]
    /// Get the monetary value (Money) for a node at a specific period.
    ///
    /// Returns the value as a Money object if the node is monetary,
    /// None if the node is scalar or not found.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period identifier
    ///
    /// Returns
    /// -------
    /// Money | None
    ///     Money value if found and monetary, None otherwise
    fn get_money(
        &self,
        node_id: &str,
        period_id: &crate::core::dates::periods::PyPeriodId,
    ) -> Option<crate::core::money::PyMoney> {
        self.inner
            .get_money(node_id, &period_id.inner)
            .map(crate::core::money::PyMoney::new)
    }

    #[pyo3(text_signature = "(self, node_id, period_id)")]
    /// Get the scalar value for a node at a specific period.
    ///
    /// Returns the value if the node is scalar, None if monetary or not found.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    /// period_id : PeriodId
    ///     Period identifier
    ///
    /// Returns
    /// -------
    /// float | None
    ///     Scalar value if found and scalar, None otherwise
    fn get_scalar(
        &self,
        node_id: &str,
        period_id: &crate::core::dates::periods::PyPeriodId,
    ) -> Option<f64> {
        self.inner.get_scalar(node_id, &period_id.inner)
    }
```

**Step 4: Update .pyi stub**

In `finstack-py/finstack/statements/evaluator/__init__.pyi`, add to `StatementResult` class:

```python
    def get_money(self, node_id: str, period_id: PeriodId) -> Money | None:
        """Get monetary value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            Money | None: Money value if monetary node, None otherwise
        """
        ...

    def get_scalar(self, node_id: str, period_id: PeriodId) -> float | None:
        """Get scalar value for a node at a specific period.

        Args:
            node_id: Node identifier
            period_id: Period identifier

        Returns:
            float | None: Scalar value if scalar node, None otherwise
        """
        ...
```

**Step 5: Run tests to verify they pass**

Run: `cd finstack-py && maturin develop --release && pytest tests/test_statements_parity.py::test_statement_result_get_money tests/test_statements_parity.py::test_statement_result_get_scalar -v`
Expected: PASS

**Step 6: Commit**

```bash
git add finstack-py/src/statements/evaluator/mod.rs finstack-py/finstack/statements/evaluator/__init__.pyi finstack-py/tests/test_statements_parity.py
git commit -m "feat(py): add get_money and get_scalar to StatementResult"
```

---

### Task 3: Add `with_market_context()` to Evaluator

**Files:**
- Modify: `finstack-py/src/statements/evaluator/mod.rs`
- Modify: `finstack-py/finstack/statements/evaluator/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_evaluator_with_market_context():
    """Test Evaluator.with_market_context returns EvaluatorWithContext."""
    from datetime import date
    from finstack.core.dates import PeriodId
    from finstack.core.market_data import MarketContext
    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("ctx_test")
    builder.periods("2025Q1..Q2", None)
    builder.value("revenue", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0))])
    model = builder.build()

    evaluator = Evaluator.new()
    market_ctx = MarketContext()
    ctx_evaluator = evaluator.with_market_context(market_ctx, date(2025, 1, 1))

    # Should be able to evaluate with the context evaluator
    results = ctx_evaluator.evaluate(model)
    assert results is not None
    assert results.get("revenue", PeriodId.quarter(2025, 1)) == pytest.approx(100.0)
```

**Step 2: Run test to verify it fails**

Run: `cd finstack-py && maturin develop --release && pytest tests/test_statements_parity.py::test_evaluator_with_market_context -v`
Expected: AttributeError: 'Evaluator' object has no attribute 'with_market_context'

**Step 3: Implement in Rust**

In `finstack-py/src/statements/evaluator/mod.rs`, add inside the `#[pymethods] impl PyEvaluator` block:

```rust
    #[pyo3(text_signature = "(self, market_ctx, as_of)")]
    /// Create an evaluator with pre-configured market context.
    ///
    /// Parameters
    /// ----------
    /// market_ctx : MarketContext
    ///     Market context with discount/forward curves
    /// as_of : date
    ///     Valuation date for pricing
    ///
    /// Returns
    /// -------
    /// EvaluatorWithContext
    ///     Evaluator with stored market context
    fn with_market_context(
        &self,
        market_ctx: &PyMarketContext,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyEvaluatorWithContext> {
        let as_of_date = py_to_date(as_of)?;
        let inner =
            finstack_statements::evaluator::Evaluator::with_market_context(
                &market_ctx.inner,
                as_of_date,
            );
        Ok(PyEvaluatorWithContext { inner })
    }
```

**Step 4: Update .pyi stub**

```python
    def with_market_context(self, market_ctx: MarketContext, as_of: date) -> EvaluatorWithContext:
        """Create evaluator with pre-configured market context.

        Args:
            market_ctx: Market context with curves
            as_of: Valuation date

        Returns:
            EvaluatorWithContext: Evaluator with stored context
        """
        ...
```

**Step 5: Run tests, commit**

---

## Phase 2: Supporting Types

### Task 4: Expose `NodeValueType` enum

**Files:**
- Modify: `finstack-py/src/statements/types/node.rs`
- Modify: `finstack-py/finstack/statements/types/__init__.pyi`
- Modify: `finstack-py/finstack/statements/__init__.pyi` (re-export)
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_node_value_type_enum():
    """Test NodeValueType enum exposure."""
    from finstack.core.currency import USD
    from finstack.statements.types import NodeValueType

    scalar = NodeValueType.SCALAR
    assert scalar is not None
    assert scalar.currency is None

    monetary = NodeValueType.monetary(USD)
    assert monetary is not None
    assert monetary.currency is not None
```

**Step 2: Implement in Rust**

In `finstack-py/src/statements/types/node.rs`, add:

```rust
use finstack_statements::types::NodeValueType;

#[pyclass(
    module = "finstack.statements.types",
    name = "NodeValueType",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyNodeValueType {
    pub(crate) inner: NodeValueType,
}

impl PyNodeValueType {
    pub(crate) fn new(inner: NodeValueType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeValueType {
    #[classattr]
    fn SCALAR() -> Self {
        Self::new(NodeValueType::Scalar)
    }

    #[staticmethod]
    #[pyo3(text_signature = "(currency)")]
    fn monetary(currency: &crate::core::currency::PyCurrency) -> Self {
        Self::new(NodeValueType::Monetary {
            currency: currency.inner,
        })
    }

    #[getter]
    fn currency(&self) -> Option<crate::core::currency::PyCurrency> {
        match &self.inner {
            NodeValueType::Monetary { currency } => {
                Some(crate::core::currency::PyCurrency::new(*currency))
            }
            NodeValueType::Scalar => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            NodeValueType::Monetary { currency } => format!("NodeValueType.monetary({})", currency),
            NodeValueType::Scalar => "NodeValueType.SCALAR".to_string(),
        }
    }
}
```

Register in the node module's `register()` function: `module.add_class::<PyNodeValueType>()?;`

**Step 3: Run tests, update stubs, commit**

---

### Task 5: Add `cs_cashflows` and `node_value_types` to StatementResult

**Files:**
- Modify: `finstack-py/src/statements/evaluator/mod.rs`
- Modify: `finstack-py/finstack/statements/evaluator/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_statement_result_node_value_types():
    """Test node_value_types property on StatementResult."""
    from finstack.core.currency import USD
    from finstack.core.dates import PeriodId
    from finstack.statements import AmountOrScalar, Evaluator, ModelBuilder

    builder = ModelBuilder.new("value_types_test")
    builder.periods("2025Q1..Q2", None)
    builder.value_money(
        "revenue",
        [(PeriodId.quarter(2025, 1), AmountOrScalar.amount(100000.0, USD))],
    )
    builder.value_scalar("margin", [(PeriodId.quarter(2025, 1), 0.35)])
    model = builder.build()

    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    value_types = results.node_value_types
    assert isinstance(value_types, dict)
    # At minimum, nodes with explicit value types should be present
```

**Step 2: Implement in Rust**

Add to `#[pymethods] impl PyStatementResult`:

```rust
    #[getter]
    /// Get node value types (monetary vs scalar).
    ///
    /// Returns
    /// -------
    /// dict[str, NodeValueType]
    ///     Map of node_id to value type
    fn node_value_types(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use crate::statements::types::node::PyNodeValueType;
        let dict = PyDict::new(py);
        for (node_id, value_type) in &self.inner.node_value_types {
            dict.set_item(node_id, PyNodeValueType::new(value_type.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get capital structure cashflows if available.
    ///
    /// Returns
    /// -------
    /// CapitalStructureCashflows | None
    ///     Capital structure cashflows if model has capital structure
    fn cs_cashflows(&self) -> Option<crate::statements::capital_structure::PyCapitalStructureCashflows> {
        self.inner
            .cs_cashflows
            .as_ref()
            .map(|cs| crate::statements::capital_structure::PyCapitalStructureCashflows::new(cs.clone()))
    }
```

**Step 3: Run tests, update stubs, commit**

---

## Phase 3: Analysis Foundation

### Task 6: Add Backtesting module

**Files:**
- Create: `finstack-py/src/statements/analysis/backtesting.rs`
- Modify: `finstack-py/src/statements/analysis/mod.rs` (add `mod backtesting;` and register)
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_backtest_forecast():
    """Test forecast backtesting metrics."""
    from finstack.statements.analysis import backtest_forecast

    actual = [100.0, 110.0, 105.0, 115.0]
    forecast = [98.0, 112.0, 104.0, 116.0]

    metrics = backtest_forecast(actual, forecast)
    assert metrics.mae > 0.0
    assert metrics.mape > 0.0
    assert metrics.rmse >= metrics.mae  # RMSE >= MAE always
    assert metrics.n == 4

    summary = metrics.summary()
    assert "MAE" in summary
    assert "MAPE" in summary
    assert "RMSE" in summary


def test_backtest_forecast_perfect():
    """Test perfect forecast has zero errors."""
    from finstack.statements.analysis import backtest_forecast

    actual = [100.0, 110.0, 120.0]
    metrics = backtest_forecast(actual, actual)
    assert metrics.mae == pytest.approx(0.0)
    assert metrics.rmse == pytest.approx(0.0)


def test_backtest_forecast_mismatched_lengths():
    """Test error on mismatched array lengths."""
    from finstack.statements.analysis import backtest_forecast

    with pytest.raises(Exception):
        backtest_forecast([1.0, 2.0], [1.0])
```

**Step 2: Implement**

Create `finstack-py/src/statements/analysis/backtesting.rs`:

```rust
//! Forecast backtesting bindings.

use crate::statements::error::stmt_to_py;
use finstack_statements::analysis::backtesting::{
    backtest_forecast as rs_backtest_forecast, ForecastMetrics,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Forecast accuracy metrics.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ForecastMetrics",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyForecastMetrics {
    inner: ForecastMetrics,
}

#[pymethods]
impl PyForecastMetrics {
    #[getter]
    /// Mean Absolute Error.
    fn mae(&self) -> f64 {
        self.inner.mae
    }

    #[getter]
    /// Mean Absolute Percentage Error.
    fn mape(&self) -> f64 {
        self.inner.mape
    }

    #[getter]
    /// Root Mean Squared Error.
    fn rmse(&self) -> f64 {
        self.inner.rmse
    }

    #[getter]
    /// Number of data points.
    fn n(&self) -> usize {
        self.inner.n
    }

    /// Format metrics as a human-readable summary.
    ///
    /// Returns
    /// -------
    /// str
    ///     Summary string
    fn summary(&self) -> String {
        self.inner.summary()
    }

    fn __repr__(&self) -> String {
        format!(
            "ForecastMetrics(mae={:.4}, mape={:.4}%, rmse={:.4}, n={})",
            self.inner.mae, self.inner.mape, self.inner.rmse, self.inner.n
        )
    }
}

#[pyfunction]
#[pyo3(signature = (actual, forecast), name = "backtest_forecast")]
/// Compute forecast error metrics by comparing actual vs forecast values.
///
/// Parameters
/// ----------
/// actual : list[float]
///     Actual observed values
/// forecast : list[float]
///     Forecasted/predicted values
///
/// Returns
/// -------
/// ForecastMetrics
///     Metrics containing MAE, MAPE, and RMSE
fn py_backtest_forecast(actual: Vec<f64>, forecast: Vec<f64>) -> PyResult<PyForecastMetrics> {
    let metrics = rs_backtest_forecast(&actual, &forecast).map_err(stmt_to_py)?;
    Ok(PyForecastMetrics { inner: metrics })
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyForecastMetrics>()?;
    module.add_function(wrap_pyfunction!(py_backtest_forecast, module)?)?;
    Ok(vec!["ForecastMetrics", "backtest_forecast"])
}
```

In `finstack-py/src/statements/analysis/mod.rs`, add `mod backtesting;` and call `backtesting::register(py, &module)?` in the register function.

**Step 3: Run tests, update stubs, commit**

---

### Task 7: Add Credit Context module

**Files:**
- Create: `finstack-py/src/statements/analysis/credit_context.rs`
- Modify: `finstack-py/src/statements/analysis/mod.rs`
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_credit_context_metrics():
    """Test CreditContextMetrics struct properties."""
    # This test requires a model with capital structure, which is more complex.
    # Focus on import availability and basic construction for now.
    from finstack.statements.analysis import CreditContextMetrics, compute_credit_context

    assert CreditContextMetrics is not None
    assert compute_credit_context is not None
```

**Step 2: Implement**

Create `finstack-py/src/statements/analysis/credit_context.rs`:

```rust
//! Credit context metrics bindings.

use crate::core::dates::periods::PyPeriodId;
use crate::statements::capital_structure::PyCapitalStructureCashflows;
use crate::statements::evaluator::PyStatementResult;
use finstack_statements::analysis::credit_context::{
    compute_credit_context as rs_compute_credit_context, CreditContextMetrics,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Per-instrument credit context metrics.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CreditContextMetrics",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCreditContextMetrics {
    pub(crate) inner: CreditContextMetrics,
}

impl PyCreditContextMetrics {
    pub(crate) fn new(inner: CreditContextMetrics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditContextMetrics {
    #[getter]
    /// DSCR by period.
    fn dscr(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .dscr
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// Interest coverage by period.
    fn interest_coverage(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .interest_coverage
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// LTV by period.
    fn ltv(&self) -> Vec<(PyPeriodId, f64)> {
        self.inner
            .ltv
            .iter()
            .map(|(p, v)| (PyPeriodId::new(*p), *v))
            .collect()
    }

    #[getter]
    /// Minimum DSCR across all periods.
    fn dscr_min(&self) -> Option<f64> {
        self.inner.dscr_min
    }

    #[getter]
    /// Minimum interest coverage across all periods.
    fn interest_coverage_min(&self) -> Option<f64> {
        self.inner.interest_coverage_min
    }

    fn __repr__(&self) -> String {
        format!(
            "CreditContextMetrics(dscr_min={:?}, icr_min={:?})",
            self.inner.dscr_min, self.inner.interest_coverage_min
        )
    }
}

#[pyfunction]
#[pyo3(
    signature = (statement, cs_cashflows, instrument_id, coverage_node, periods, reference_value=None),
    name = "compute_credit_context"
)]
/// Compute credit context metrics for a specific instrument.
///
/// Parameters
/// ----------
/// statement : StatementResult
///     Evaluated statement results
/// cs_cashflows : CapitalStructureCashflows
///     Capital structure cashflows
/// instrument_id : str
///     Instrument to compute metrics for
/// coverage_node : str
///     Statement node for coverage numerator (e.g. "ebitda")
/// periods : list[Period]
///     Periods over which to compute metrics
/// reference_value : float | None
///     Optional reference value for LTV (e.g. enterprise value)
///
/// Returns
/// -------
/// CreditContextMetrics
///     Credit metrics (DSCR, interest coverage, LTV)
fn py_compute_credit_context(
    statement: &PyStatementResult,
    cs_cashflows: &PyCapitalStructureCashflows,
    instrument_id: &str,
    coverage_node: &str,
    periods: Vec<crate::core::dates::periods::PyPeriod>,
    reference_value: Option<f64>,
) -> PyCreditContextMetrics {
    let rs_periods: Vec<finstack_core::dates::Period> =
        periods.iter().map(|p| p.inner).collect();
    let metrics = rs_compute_credit_context(
        &statement.inner,
        &cs_cashflows.inner,
        instrument_id,
        coverage_node,
        &rs_periods,
        reference_value,
    );
    PyCreditContextMetrics::new(metrics)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCreditContextMetrics>()?;
    module.add_function(wrap_pyfunction!(py_compute_credit_context, module)?)?;
    Ok(vec!["CreditContextMetrics", "compute_credit_context"])
}
```

**Step 3: Run tests, update stubs, commit**

---

### Task 8: Add DCF Corporate Valuation module

**Files:**
- Create: `finstack-py/src/statements/analysis/corporate.rs`
- Modify: `finstack-py/src/statements/analysis/mod.rs`
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_evaluate_dcf_basic():
    """Test basic DCF evaluation."""
    from finstack.core.dates import PeriodId
    from finstack.statements import AmountOrScalar, ModelBuilder
    from finstack.statements.analysis import evaluate_dcf
    from finstack.valuations.instruments import TerminalValueSpec

    builder = ModelBuilder.new("dcf_test")
    builder.periods("2025Q1..Q4", None)
    builder.value(
        "ufcf",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110000.0)),
            (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(120000.0)),
            (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(130000.0)),
        ],
    )
    builder.with_meta("currency", "USD")
    model = builder.build()

    tv = TerminalValueSpec.gordon_growth(growth_rate=0.02)
    result = evaluate_dcf(model, wacc=0.10, terminal_value=tv)

    assert result.equity_value is not None
    assert result.enterprise_value is not None
    assert result.equity_value.amount > 0.0
    assert result.enterprise_value.amount > result.net_debt.amount


def test_dcf_options():
    """Test DcfOptions construction."""
    from finstack.statements.analysis import DcfOptions

    opts = DcfOptions(mid_year_convention=True, shares_outstanding=1000000.0)
    assert opts.mid_year_convention is True
    assert opts.shares_outstanding == pytest.approx(1000000.0)
```

**Step 2: Implement**

Create `finstack-py/src/statements/analysis/corporate.rs`:

```rust
//! Corporate DCF valuation bindings.

use crate::core::money::PyMoney;
use crate::statements::error::stmt_to_py;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::analysis::corporate::{
    CorporateValuationResult, DcfOptions,
    evaluate_dcf as rs_evaluate_dcf,
    evaluate_dcf_with_market as rs_evaluate_dcf_with_market,
    evaluate_dcf_with_options as rs_evaluate_dcf_with_options,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Optional configuration for DCF valuation.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "DcfOptions",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyDcfOptions {
    pub(crate) inner: DcfOptions,
}

#[pymethods]
impl PyDcfOptions {
    #[new]
    #[pyo3(signature = (*, mid_year_convention=false, shares_outstanding=None))]
    fn new(mid_year_convention: bool, shares_outstanding: Option<f64>) -> Self {
        Self {
            inner: DcfOptions {
                mid_year_convention,
                equity_bridge: None,
                shares_outstanding,
                valuation_discounts: None,
            },
        }
    }

    #[getter]
    fn mid_year_convention(&self) -> bool {
        self.inner.mid_year_convention
    }

    #[getter]
    fn shares_outstanding(&self) -> Option<f64> {
        self.inner.shares_outstanding
    }

    fn __repr__(&self) -> String {
        format!(
            "DcfOptions(mid_year={}, shares={:?})",
            self.inner.mid_year_convention, self.inner.shares_outstanding
        )
    }
}

/// Corporate valuation result from DCF analysis.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "CorporateValuationResult",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCorporateValuationResult {
    pub(crate) inner: CorporateValuationResult,
}

impl PyCorporateValuationResult {
    pub(crate) fn new(inner: CorporateValuationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCorporateValuationResult {
    #[getter]
    fn equity_value(&self) -> PyMoney {
        PyMoney::new(self.inner.equity_value)
    }

    #[getter]
    fn enterprise_value(&self) -> PyMoney {
        PyMoney::new(self.inner.enterprise_value)
    }

    #[getter]
    fn net_debt(&self) -> PyMoney {
        PyMoney::new(self.inner.net_debt)
    }

    #[getter]
    fn terminal_value_pv(&self) -> PyMoney {
        PyMoney::new(self.inner.terminal_value_pv)
    }

    #[getter]
    fn equity_value_per_share(&self) -> Option<f64> {
        self.inner.equity_value_per_share
    }

    #[getter]
    fn diluted_shares(&self) -> Option<f64> {
        self.inner.diluted_shares
    }

    fn __repr__(&self) -> String {
        format!(
            "CorporateValuationResult(ev={}, equity={})",
            self.inner.enterprise_value, self.inner.equity_value
        )
    }
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None),
    name = "evaluate_dcf"
)]
fn py_evaluate_dcf(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
) -> PyResult<PyCorporateValuationResult> {
    let result = rs_evaluate_dcf(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None, options=None),
    name = "evaluate_dcf_with_options"
)]
fn py_evaluate_dcf_with_options(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: Option<&PyDcfOptions>,
) -> PyResult<PyCorporateValuationResult> {
    let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
    let result = rs_evaluate_dcf_with_options(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
        &opts,
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

#[pyfunction]
#[pyo3(
    signature = (model, wacc, terminal_value, ufcf_node="ufcf", net_debt_override=None, options=None, market=None),
    name = "evaluate_dcf_with_market"
)]
fn py_evaluate_dcf_with_market(
    model: &PyFinancialModelSpec,
    wacc: f64,
    terminal_value: &crate::valuations::instruments::equity::dcf::PyTerminalValueSpec,
    ufcf_node: &str,
    net_debt_override: Option<f64>,
    options: Option<&PyDcfOptions>,
    market: Option<&crate::core::market_data::context::PyMarketContext>,
) -> PyResult<PyCorporateValuationResult> {
    let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
    let result = rs_evaluate_dcf_with_market(
        &model.inner,
        wacc,
        terminal_value.inner.clone(),
        ufcf_node,
        net_debt_override,
        &opts,
        market.map(|m| &m.inner),
    )
    .map_err(stmt_to_py)?;
    Ok(PyCorporateValuationResult::new(result))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDcfOptions>()?;
    module.add_class::<PyCorporateValuationResult>()?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf, module)?)?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf_with_options, module)?)?;
    module.add_function(wrap_pyfunction!(py_evaluate_dcf_with_market, module)?)?;
    Ok(vec![
        "DcfOptions",
        "CorporateValuationResult",
        "evaluate_dcf",
        "evaluate_dcf_with_options",
        "evaluate_dcf_with_market",
    ])
}
```

**Note:** The exact import path for `PyTerminalValueSpec` depends on the existing valuations binding structure. Check `finstack-py/src/valuations/instruments/equity/dcf.rs` for the actual struct name and path.

**Step 3: Run tests, update stubs, commit**

---

## Phase 4: Analysis Advanced

### Task 9: Add Monte Carlo Config

**Files:**
- Modify: `finstack-py/src/statements/evaluator/mod.rs` (or new analysis file)
- Modify: `finstack-py/finstack/statements/evaluator/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_monte_carlo_config():
    """Test MonteCarloConfig construction."""
    from finstack.statements.analysis import MonteCarloConfig

    config = MonteCarloConfig(n_paths=1000, seed=42)
    assert config.n_paths == 1000
    assert config.seed == 42

    config2 = config.with_percentiles([0.1, 0.5, 0.9])
    assert config2.percentiles == pytest.approx([0.1, 0.5, 0.9])
```

**Step 2: Implement**

Add `PyMonteCarloConfig` wrapper following existing patterns. The Rust `MonteCarloConfig` is already imported in the evaluator module.

**Step 3: Run tests, update stubs, commit**

---

### Task 10: Add Covenant Analysis module

**Files:**
- Create: `finstack-py/src/statements/analysis/covenants.rs`
- Modify: `finstack-py/src/statements/analysis/mod.rs`
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_forecast_breaches_import():
    """Test covenant analysis functions are importable."""
    from finstack.statements.analysis import forecast_breaches, forecast_covenant

    assert forecast_breaches is not None
    assert forecast_covenant is not None
```

**Step 2: Implement**

Create `finstack-py/src/statements/analysis/covenants.rs` wrapping `finstack_statements::analysis::covenants::forecast_covenant`, `forecast_breaches`, and `forecast_covenants`. Reuse `PyCovenantSpec`, `PyCovenantForecastConfig`, `PyCovenantEngine` from `crate::valuations::covenants`.

**Step 3: Run tests, update stubs, commit**

---

### Task 11: Add Corporate Orchestrator module

**Files:**
- Create: `finstack-py/src/statements/analysis/orchestrator.rs`
- Modify: `finstack-py/src/statements/analysis/mod.rs`
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi`
- Test: `finstack-py/tests/test_statements_parity.py`

**Step 1: Write failing test**

```python
def test_corporate_analysis_builder():
    """Test CorporateAnalysisBuilder fluent API."""
    from finstack.core.dates import PeriodId
    from finstack.statements import AmountOrScalar, ModelBuilder
    from finstack.statements.analysis import CorporateAnalysisBuilder
    from finstack.valuations.instruments import TerminalValueSpec

    builder = ModelBuilder.new("corp_test")
    builder.periods("2025Q1..Q4", None)
    builder.value(
        "ufcf",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110000.0)),
            (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(120000.0)),
            (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(130000.0)),
        ],
    )
    builder.compute("ebitda", "ufcf * 1.5")
    builder.with_meta("currency", "USD")
    model = builder.build()

    tv = TerminalValueSpec.gordon_growth(growth_rate=0.02)
    analysis = (
        CorporateAnalysisBuilder(model)
        .dcf(0.10, tv)
        .net_debt_override(50000.0)
        .analyze()
    )

    assert analysis.statement is not None
    assert analysis.equity is not None
    assert analysis.equity.equity_value.amount > 0.0
```

**Step 2: Implement**

Create `finstack-py/src/statements/analysis/orchestrator.rs` wrapping `CorporateAnalysisBuilder`, `CorporateAnalysis`, and `CreditInstrumentAnalysis`. The builder uses a fluent API pattern — store the Rust builder and call `.analyze()` to produce results.

**Step 3: Run tests, update stubs, commit**

---

## Phase 5: Stubs & Polish

### Task 12: Update all .pyi stubs

**Files:**
- Modify: `finstack-py/finstack/statements/__init__.pyi` (re-exports)
- Modify: `finstack-py/finstack/statements/types/__init__.pyi` (NodeValueType)
- Modify: `finstack-py/finstack/statements/evaluator/__init__.pyi` (get_money, get_scalar, cs_cashflows, node_value_types, with_market_context)
- Modify: `finstack-py/finstack/statements/analysis/__init__.pyi` (all new analysis types)
- Modify: `finstack-py/finstack/statements/builder/__init__.pyi` (missing method stubs)

**Step 1: Audit each .pyi file against implemented methods**

For each module, run the Python binding and check `dir()` output against the .pyi stubs.

**Step 2: Add missing stubs**

Follow the existing pattern: `from __future__ import annotations`, `__all__`, class with properties and methods, docstrings on `__init__` and methods.

**Step 3: Add missing builder stubs**

Add stubs for existing but undocumented builder methods:
- `where_clause(where_clause: str) -> ModelBuilder`
- `add_roll_forward(item_id: str, additions: str, disposals: str) -> ModelBuilder`
- `add_vintage_buildup(vintage_id: str, component_nodes: list[str]) -> ModelBuilder`
- `add_noi_buildup(...)` and other real estate methods
- `add_registry_metrics(...)`

**Step 4: Run full test suite**

Run: `cd finstack-py && maturin develop --release && pytest tests/ -v`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add finstack-py/finstack/statements/
git commit -m "docs(py): update .pyi stubs for full statements parity"
```

---

### Task 13: Final verification

**Step 1: Run full test suite**

```bash
cd finstack-py && maturin develop --release && pytest tests/ -v --tb=short
```

**Step 2: Run cargo clippy on bindings**

```bash
cd finstack-py && cargo clippy --all-features -- -D warnings
```

**Step 3: Verify Python imports work**

```python
python -c "
from finstack.statements.analysis import (
    backtest_forecast, ForecastMetrics,
    compute_credit_context, CreditContextMetrics,
    evaluate_dcf, evaluate_dcf_with_options, evaluate_dcf_with_market,
    DcfOptions, CorporateValuationResult,
    CorporateAnalysisBuilder, CorporateAnalysis, CreditInstrumentAnalysis,
    MonteCarloConfig,
    forecast_breaches, forecast_covenant,
)
from finstack.statements.types import NodeValueType
print('All imports successful!')
"
```

**Step 4: Final commit if needed**

---

## Summary

| Task | Phase | Items | New Files |
|------|-------|-------|-----------|
| 1 | Core | `is_amount`, `as_money()` | 0 |
| 2 | Core | `get_money()`, `get_scalar()` | 0 |
| 3 | Core | `with_market_context()` | 0 |
| 4 | Types | `NodeValueType` enum | 0 |
| 5 | Types | `cs_cashflows`, `node_value_types` | 0 |
| 6 | Analysis | Backtesting | 1 |
| 7 | Analysis | Credit Context | 1 |
| 8 | Analysis | DCF Valuation | 1 |
| 9 | Analysis | Monte Carlo Config | 0 |
| 10 | Analysis | Covenant Analysis | 1 |
| 11 | Analysis | Corporate Orchestrator | 1 |
| 12 | Stubs | .pyi updates | 0 |
| 13 | Verify | Full test suite | 0 |
