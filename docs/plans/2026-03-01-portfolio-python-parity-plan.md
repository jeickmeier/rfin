# Portfolio Python Binding Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Achieve P0-P2 parity between the Rust `finstack-portfolio` public API and the Python bindings (~47 items across 5 streams).

**Architecture:** 5 independent work streams touching non-overlapping files. Each stream is a self-contained unit: modify Rust binding code, update `.pyi` stubs, add parity tests, build, verify. Streams can run in parallel via subagents.

**Tech Stack:** Rust (PyO3 0.28), Python (maturin build), pytest, polars

**Build command:** `maturin develop -m finstack-py/Cargo.toml`
**Test command:** `pytest finstack-py/tests/parity/test_portfolio_parity.py -v`

---

## Stream 1: Scenario Return Values (P0)

### Task 1.1: Update apply_scenario to return tuple

**Files:**
- Modify: `finstack-py/src/portfolio/scenarios.rs`
- Modify: `finstack-py/finstack/portfolio/scenarios.pyi`
- Modify: `finstack-py/finstack/portfolio/__init__.pyi`
- Test: `finstack-py/tests/parity/test_portfolio_parity.py`

**Step 1: Write failing test**

Add to `finstack-py/tests/parity/test_portfolio_parity.py`:

```python
class TestScenarioParity:
    """Test scenario return values match Rust API."""

    def test_apply_scenario_returns_tuple(self, standard_market, base_date):
        """apply_scenario must return (Portfolio, MarketContext, ApplicationReport)."""
        from finstack.portfolio import (
            PortfolioBuilder, Entity, Position, PositionUnit,
            apply_scenario,
        )
        from finstack.scenarios import ScenarioSpec, ApplicationReport
        from finstack.core.currency import USD
        from finstack.valuations.instruments import Equity

        entity = Entity("E1")
        equity = Equity.builder("EQ-1").ticker("TEST").currency(USD).price(100.0).build()
        pos = Position("P1", "E1", "EQ-1", equity, 100.0, PositionUnit.UNITS)
        portfolio = (
            PortfolioBuilder("TEST")
            .base_ccy(USD)
            .as_of(base_date)
            .entity(entity)
            .position(pos)
            .build()
        )

        # Build a simple parallel-shift scenario
        scenario = ScenarioSpec.builder("test_shift").build()

        result = apply_scenario(portfolio, scenario, standard_market)
        assert isinstance(result, tuple), "apply_scenario must return a tuple"
        assert len(result) == 3, "tuple must have 3 elements"
        portfolio_out, market_out, report = result
        assert isinstance(report, ApplicationReport)
        assert isinstance(report.operations_applied, int)
        assert isinstance(report.warnings, list)

    def test_apply_and_revalue_returns_tuple(self, standard_market, base_date):
        """apply_and_revalue must return (PortfolioValuation, ApplicationReport)."""
        from finstack.portfolio import (
            PortfolioBuilder, Entity, Position, PositionUnit,
            apply_and_revalue, PortfolioValuation,
        )
        from finstack.scenarios import ScenarioSpec, ApplicationReport
        from finstack.core.currency import USD
        from finstack.valuations.instruments import Equity

        entity = Entity("E1")
        equity = Equity.builder("EQ-1").ticker("TEST").currency(USD).price(100.0).build()
        pos = Position("P1", "E1", "EQ-1", equity, 100.0, PositionUnit.UNITS)
        portfolio = (
            PortfolioBuilder("TEST")
            .base_ccy(USD)
            .as_of(base_date)
            .entity(entity)
            .position(pos)
            .build()
        )

        scenario = ScenarioSpec.builder("test_shift").build()
        result = apply_and_revalue(portfolio, scenario, standard_market)
        assert isinstance(result, tuple), "apply_and_revalue must return a tuple"
        assert len(result) == 2
        valuation, report = result
        assert isinstance(valuation, PortfolioValuation)
        assert isinstance(report, ApplicationReport)
```

**Step 2: Run test to verify it fails**

Run: `pytest finstack-py/tests/parity/test_portfolio_parity.py::TestScenarioParity -v`
Expected: FAIL — currently returns single object, not tuple

**Step 3: Modify Rust binding**

In `finstack-py/src/portfolio/scenarios.rs`, import `PyApplicationReport` and `PyMarketContext`, and return tuples:

```rust
use crate::scenarios::reports::PyApplicationReport;

// apply_scenario: return (Portfolio, MarketContext, ApplicationReport)
fn py_apply_scenario(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
) -> PyResult<(PyPortfolio, PyMarketContext, PyApplicationReport)> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let scenario_inner = scenario.extract::<PyRef<PyScenarioSpec>>()?.inner.clone();
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let (transformed, stressed_market, report) =
        apply_scenario(&portfolio_inner, &scenario_inner, &market_ctx.inner)
            .map_err(portfolio_to_py)?;

    Ok((
        PyPortfolio::new(transformed),
        PyMarketContext::new(stressed_market),
        PyApplicationReport::new(report),
    ))
}

// apply_and_revalue: return (PortfolioValuation, ApplicationReport)
fn py_apply_and_revalue(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<(PyPortfolioValuation, PyApplicationReport)> {
    // ... extract inputs ...
    let (valuation, report) =
        apply_and_revalue(&portfolio_inner, &scenario_inner, &market_ctx.inner, &cfg)
            .map_err(portfolio_to_py)?;

    Ok((
        PyPortfolioValuation::new(valuation),
        PyApplicationReport::new(report),
    ))
}
```

Also add `ApplicationReport` to the register function's export list.

**Step 4: Update .pyi stubs**

In `finstack-py/finstack/portfolio/scenarios.pyi`:

```python
from finstack.scenarios import ApplicationReport
from finstack.core.market_data.context import MarketContext

def apply_scenario(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
) -> tuple[Portfolio, MarketContext, ApplicationReport]: ...

def apply_and_revalue(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
    config: FinstackConfig | None = None,
) -> tuple[PortfolioValuation, ApplicationReport]: ...
```

Update `__init__.pyi` to re-export `ApplicationReport`.

**Step 5: Build and run tests**

Run: `maturin develop -m finstack-py/Cargo.toml && pytest finstack-py/tests/parity/test_portfolio_parity.py::TestScenarioParity -v`
Expected: PASS

**Step 6: Commit**

```bash
git add finstack-py/src/portfolio/scenarios.rs finstack-py/finstack/portfolio/scenarios.pyi finstack-py/finstack/portfolio/__init__.pyi finstack-py/tests/parity/test_portfolio_parity.py
git commit -m "feat(py): return full tuples from apply_scenario/apply_and_revalue (BREAKING)"
```

---

## Stream 2: Valuation & Metrics Enhancement (P0-P1)

### Task 2.1: Add additional_metrics and replace_standard_metrics to PortfolioValuationOptions

**Files:**
- Modify: `finstack-py/src/portfolio/valuation.rs`
- Modify: `finstack-py/finstack/portfolio/valuation.pyi`
- Test: `finstack-py/tests/parity/test_portfolio_parity.py`

**Step 1: Write failing test**

```python
class TestValuationOptionsParity:
    """Test PortfolioValuationOptions fields."""

    def test_additional_metrics(self):
        from finstack.portfolio import PortfolioValuationOptions
        from finstack.valuations.metrics import MetricId

        opts = PortfolioValuationOptions(
            additional_metrics=[MetricId.DURATION_MOD, MetricId.CONVEXITY]
        )
        assert opts.additional_metrics is not None
        assert len(opts.additional_metrics) == 2
        assert opts.replace_standard_metrics is False

    def test_replace_standard_metrics(self):
        from finstack.portfolio import PortfolioValuationOptions
        from finstack.valuations.metrics import MetricId

        opts = PortfolioValuationOptions(
            additional_metrics=[MetricId.YTM],
            replace_standard_metrics=True,
        )
        assert opts.replace_standard_metrics is True

    def test_defaults_unchanged(self):
        from finstack.portfolio import PortfolioValuationOptions

        opts = PortfolioValuationOptions()
        assert opts.strict_risk is False
        assert opts.additional_metrics is None
        assert opts.replace_standard_metrics is False
```

**Step 2: Run test to verify it fails**

Run: `pytest finstack-py/tests/parity/test_portfolio_parity.py::TestValuationOptionsParity -v`
Expected: FAIL — constructor doesn't accept `additional_metrics`

**Step 3: Modify Rust binding**

In `finstack-py/src/portfolio/valuation.rs`, update the `PyPortfolioValuationOptions` constructor:

```rust
use crate::valuations::metrics::PyMetricId;

#[pymethods]
impl PyPortfolioValuationOptions {
    #[new]
    #[pyo3(signature = (*, strict_risk=false, additional_metrics=None, replace_standard_metrics=false))]
    fn new_py(
        strict_risk: bool,
        additional_metrics: Option<Vec<PyRef<PyMetricId>>>,
        replace_standard_metrics: bool,
    ) -> Self {
        let metrics = additional_metrics.map(|ids| {
            ids.iter().map(|m| m.inner).collect()
        });
        Self::new(PortfolioValuationOptions {
            strict_risk,
            additional_metrics: metrics,
            replace_standard_metrics,
        })
    }

    #[getter]
    fn strict_risk(&self) -> bool { self.inner.strict_risk }

    #[getter]
    fn additional_metrics(&self) -> Option<Vec<PyMetricId>> {
        self.inner.additional_metrics.as_ref().map(|ids| {
            ids.iter().map(|&id| PyMetricId::new(id)).collect()
        })
    }

    #[getter]
    fn replace_standard_metrics(&self) -> bool {
        self.inner.replace_standard_metrics
    }
}
```

**Step 4: Update .pyi stub**

In `finstack-py/finstack/portfolio/valuation.pyi`, update the class:

```python
from finstack.valuations.metrics import MetricId

class PortfolioValuationOptions:
    def __init__(
        self,
        *,
        strict_risk: bool = False,
        additional_metrics: list[MetricId] | None = None,
        replace_standard_metrics: bool = False,
    ) -> None: ...
    @property
    def strict_risk(self) -> bool: ...
    @property
    def additional_metrics(self) -> list[MetricId] | None: ...
    @property
    def replace_standard_metrics(self) -> bool: ...
```

**Step 5: Build and run tests**

Run: `maturin develop -m finstack-py/Cargo.toml && pytest finstack-py/tests/parity/test_portfolio_parity.py::TestValuationOptionsParity -v`
Expected: PASS

**Step 6: Commit**

```bash
git add finstack-py/src/portfolio/valuation.rs finstack-py/finstack/portfolio/valuation.pyi finstack-py/tests/parity/test_portfolio_parity.py
git commit -m "feat(py): expose additional_metrics and replace_standard_metrics on PortfolioValuationOptions"
```

### Task 2.2: Expose PositionValue.valuation_result

**Files:**
- Modify: `finstack-py/src/portfolio/valuation.rs`
- Modify: `finstack-py/finstack/portfolio/valuation.pyi`
- Test: `finstack-py/tests/parity/test_portfolio_parity.py`

**Step 1: Write failing test**

```python
def test_position_value_has_valuation_result(self, standard_market, base_date):
    """PositionValue should expose the underlying ValuationResult."""
    from finstack.portfolio import (
        PortfolioBuilder, Entity, Position, PositionUnit, value_portfolio,
    )
    from finstack.core.currency import USD
    from finstack.valuations.instruments import Equity

    entity = Entity("E1")
    equity = Equity.builder("EQ-1").ticker("TEST").currency(USD).price(100.0).build()
    pos = Position("P1", "E1", "EQ-1", equity, 100.0, PositionUnit.UNITS)
    portfolio = (
        PortfolioBuilder("TEST").base_ccy(USD).as_of(base_date)
        .entity(entity).position(pos).build()
    )
    valuation = value_portfolio(portfolio, standard_market)
    pv = valuation.get_position_value("P1")
    assert pv is not None

    result = pv.valuation_result
    # ValuationResult should be present for valued positions
    if result is not None:
        from finstack.valuations.results import ValuationResult
        assert isinstance(result, ValuationResult)
        assert result.instrument_id == "EQ-1"
```

**Step 2: Run test to verify it fails**

Expected: FAIL — `valuation_result` attribute doesn't exist

**Step 3: Implement getter**

In `finstack-py/src/portfolio/valuation.rs`, add to `PyPositionValue`:

```rust
use crate::valuations::results::PyValuationResult;

#[getter]
fn valuation_result(&self) -> Option<PyValuationResult> {
    self.inner.valuation_result.as_ref().map(|vr| PyValuationResult::new(vr.clone()))
}
```

**Step 4: Update .pyi stub**

```python
from finstack.valuations.results import ValuationResult

class PositionValue:
    # ... existing ...
    @property
    def valuation_result(self) -> ValuationResult | None:
        """Full valuation result if available (includes metrics, cashflows, covenants)."""
        ...
```

**Step 5: Build and run tests**

Run: `maturin develop -m finstack-py/Cargo.toml && pytest finstack-py/tests/parity/test_portfolio_parity.py -k "valuation_result" -v`
Expected: PASS

**Step 6: Commit**

```bash
git add finstack-py/src/portfolio/valuation.rs finstack-py/finstack/portfolio/valuation.pyi finstack-py/tests/parity/test_portfolio_parity.py
git commit -m "feat(py): expose PositionValue.valuation_result"
```

---

## Stream 3: Optimization Diagnostics (P1-P2)

### Task 3.1: Add missing fields to OptimizationResult

**Files:**
- Modify: `finstack-py/src/portfolio/optimization.rs`
- Modify: `finstack-py/finstack/portfolio/optimization.pyi`
- Test: `finstack-py/tests/parity/test_portfolio_parity.py`

**Step 1: Write failing test**

```python
class TestOptimizationResultParity:
    """Test OptimizationResult exposes all diagnostic fields."""

    def test_implied_quantities_property(self):
        """OptimizationResult must have implied_quantities."""
        # Build a minimal optimization and check the field exists
        result = self._run_simple_optimization()
        assert hasattr(result, 'implied_quantities')
        assert isinstance(result.implied_quantities, dict)

    def test_metric_values_property(self):
        result = self._run_simple_optimization()
        assert hasattr(result, 'metric_values')
        assert isinstance(result.metric_values, dict)

    def test_dual_values_property(self):
        result = self._run_simple_optimization()
        assert hasattr(result, 'dual_values')
        assert isinstance(result.dual_values, dict)

    def test_constraint_slacks_property(self):
        result = self._run_simple_optimization()
        assert hasattr(result, 'constraint_slacks')
        assert isinstance(result.constraint_slacks, dict)

    def test_meta_property(self):
        result = self._run_simple_optimization()
        assert hasattr(result, 'meta')
        assert isinstance(result.meta, dict)
```

**Step 2: Run test to verify it fails**

Expected: FAIL — attributes don't exist

**Step 3: Add getters to PyOptimizationResult**

In `finstack-py/src/portfolio/optimization.rs`, add to `impl PyOptimizationResult`:

```rust
#[getter]
fn implied_quantities(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    map_weights(py, &self.inner.implied_quantities)
}

#[getter]
fn metric_values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (k, v) in &self.inner.metric_values {
        dict.set_item(k.as_str(), *v)?;
    }
    Ok(dict.into())
}

#[getter]
fn dual_values(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (k, v) in &self.inner.dual_values {
        dict.set_item(k.as_str(), *v)?;
    }
    Ok(dict.into())
}

#[getter]
fn constraint_slacks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (k, v) in &self.inner.constraint_slacks {
        dict.set_item(k.as_str(), *v)?;
    }
    Ok(dict.into())
}

#[getter]
fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    pythonize(py, &self.inner.meta).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("meta serialization failed: {e}"))
    })
}
```

**Step 4: Update .pyi stub**

Add properties to `OptimizationResult` in `optimization.pyi`.

**Step 5: Build and run tests**

Expected: PASS

**Step 6: Commit**

```bash
git commit -m "feat(py): expose implied_quantities, metric_values, dual_values, constraint_slacks, meta on OptimizationResult"
```

### Task 3.2: Add OptimizationStatus variant access

**Files:**
- Modify: `finstack-py/src/portfolio/optimization.rs`
- Modify: `finstack-py/finstack/portfolio/optimization.pyi`

**Step 1: Write failing test**

```python
def test_optimization_status_variant_access(self):
    """OptimizationStatus should expose variant-specific data."""
    from finstack.portfolio.optimization import OptimizationStatus

    result = self._run_simple_optimization()
    status = result.status
    assert hasattr(status, 'status_name')
    assert status.status_name in ('Optimal', 'FeasibleButSuboptimal', 'Infeasible', 'Unbounded', 'Error')
    # For a feasible result, conflicting_constraints should be None
    assert status.conflicting_constraints is None or isinstance(status.conflicting_constraints, list)
    assert status.error_message is None or isinstance(status.error_message, str)
```

**Step 2: Run to verify failure, then implement**

Add to `PyOptimizationStatus`:

```rust
#[getter]
fn status_name(&self) -> &str {
    match &self.inner {
        OptimizationStatus::Optimal => "Optimal",
        OptimizationStatus::FeasibleButSuboptimal => "FeasibleButSuboptimal",
        OptimizationStatus::Infeasible { .. } => "Infeasible",
        OptimizationStatus::Unbounded => "Unbounded",
        OptimizationStatus::Error { .. } => "Error",
    }
}

#[getter]
fn conflicting_constraints(&self) -> Option<Vec<String>> {
    match &self.inner {
        OptimizationStatus::Infeasible { conflicting_constraints } => {
            Some(conflicting_constraints.clone())
        }
        _ => None,
    }
}

#[getter]
fn error_message(&self) -> Option<String> {
    match &self.inner {
        OptimizationStatus::Error { message } => Some(message.clone()),
        _ => None,
    }
}
```

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): expose OptimizationStatus variant data (status_name, conflicting_constraints, error_message)"
```

### Task 3.3: Bind MaxYieldWithCccLimitResult as typed class

**Files:**
- Modify: `finstack-py/src/portfolio/optimization.rs`
- Modify: `finstack-py/finstack/portfolio/optimization.pyi`

**Step 1: Write failing test**

```python
def test_max_yield_returns_typed_result(self):
    """optimize_max_yield_with_ccc_limit should return MaxYieldWithCccLimitResult."""
    from finstack.portfolio.optimization import (
        optimize_max_yield_with_ccc_limit, MaxYieldWithCccLimitResult,
    )
    result = optimize_max_yield_with_ccc_limit(portfolio, market_context)
    assert isinstance(result, MaxYieldWithCccLimitResult)
    assert hasattr(result, 'status')
    assert hasattr(result, 'objective_value')
    assert hasattr(result, 'ccc_weight')
    assert hasattr(result, 'optimal_weights')
```

**Step 2: Implement PyMaxYieldWithCccLimitResult**

Add a new `#[pyclass]` wrapping `MaxYieldWithCccLimitResult` with all field getters. Update `optimize_max_yield_with_ccc_limit` to return this class instead of `dict`.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): bind MaxYieldWithCccLimitResult as typed class"
```

### Task 3.4: Add PortfolioOptimizationProblem readable getters

Add `#[getter]` for `portfolio`, `objective`, `constraints`, `trade_universe` on `PyPortfolioOptimizationProblem`.

**Commit:** `feat(py): add readable getters to PortfolioOptimizationProblem`

### Task 3.5: Bind DefaultLpOptimizer

**Step 1: Write failing test**

```python
def test_default_lp_optimizer(self):
    from finstack.portfolio.optimization import DefaultLpOptimizer

    optimizer = DefaultLpOptimizer(tolerance=1e-6, max_iterations=5000)
    assert optimizer.tolerance == 1e-6
    assert optimizer.max_iterations == 5000

    result = optimizer.optimize(problem, market_context)
    assert result.status.is_feasible()
```

**Step 2: Implement**

Add `PyDefaultLpOptimizer` wrapping `DefaultLpOptimizer` with `#[new]`, getters for `tolerance`/`max_iterations`, and `optimize()` method.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): bind DefaultLpOptimizer with configurable tolerance and max_iterations"
```

---

## Stream 4: Core Types & Grouping (P1-P2)

### Task 4.1: Add Position.instrument property

**Files:**
- Modify: `finstack-py/src/valuations/instruments/mod.rs` — add `instrument_to_py` reverse conversion
- Modify: `finstack-py/src/portfolio/types.rs` — add `#[getter] fn instrument()`
- Modify: `finstack-py/finstack/portfolio/types.pyi`

**Step 1: Write failing test**

```python
def test_position_instrument_property(self):
    """Position.instrument should return the instrument object."""
    from finstack.portfolio import Entity, Position, PositionUnit
    from finstack.core.currency import USD
    from finstack.valuations.instruments import Equity

    equity = Equity.builder("EQ-1").ticker("TEST").currency(USD).price(100.0).build()
    pos = Position("P1", "E1", "EQ-1", equity, 100.0, PositionUnit.UNITS)
    inst = pos.instrument
    assert inst is not None
    assert inst.instrument_id == "EQ-1"
```

**Step 2: Create instrument_to_py helper**

In `finstack-py/src/valuations/instruments/mod.rs`, add a reverse conversion macro and function:

```rust
macro_rules! try_downcast_to_py {
    ($arc:expr, $py:expr, $rust_type:ty, $py_type:ident) => {
        if let Some(concrete) = $arc.as_any().downcast_ref::<$rust_type>() {
            return Ok($py_type { inner: Arc::new(concrete.clone()) }.into_pyobject($py)?.into_any().unbind());
        }
    };
}

/// Convert an Arc<dyn Instrument> back to the appropriate Python wrapper.
pub(crate) fn instrument_to_py(py: Python<'_>, inst: &Arc<dyn Instrument>) -> PyResult<Py<PyAny>> {
    use finstack_valuations::instruments::*;

    try_downcast_to_py!(inst, py, Bond, PyBond);
    try_downcast_to_py!(inst, py, Equity, PyEquity);
    try_downcast_to_py!(inst, py, Deposit, PyDeposit);
    // ... continue for all instrument types matching extract_instrument ...

    Err(PyTypeError::new_err(format!(
        "Cannot convert instrument '{}' back to Python type",
        inst.instrument_id()
    )))
}
```

**Note:** The `Instrument` trait must implement `as_any()` returning `&dyn Any`. If it doesn't, use `InstrumentType::key()` to dispatch instead, or store the original Python object in the Position wrapper.

**Step 3: Add getter to PyPosition**

In `finstack-py/src/portfolio/types.rs`:

```rust
use crate::valuations::instruments::instrument_to_py;

#[getter]
fn instrument(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
    instrument_to_py(py, &self.inner.instrument)
}
```

**Step 4: Update .pyi stub, build, test, commit**

```bash
git commit -m "feat(py): expose Position.instrument property with reverse instrument conversion"
```

### Task 4.2: Add Book methods

**Files:**
- Modify: `finstack-py/src/portfolio/book.rs`
- Modify: `finstack-py/finstack/portfolio/types.pyi`

**Step 1: Write failing test**

```python
class TestBookMethodsParity:
    def test_is_root(self):
        from finstack.portfolio import Book
        root = Book("ROOT", name="Root Book")
        child = Book("CHILD", name="Child", parent_id="ROOT")
        assert root.is_root() is True
        assert child.is_root() is False

    def test_contains_position(self):
        from finstack.portfolio import Book, PortfolioBuilder, Entity, Position, PositionUnit
        # Build a book, add a position, check contains
        book = Book("B1")
        book.add_position("P1")
        assert book.contains_position("P1") is True
        assert book.contains_position("P2") is False

    def test_add_remove_child(self):
        from finstack.portfolio import Book
        book = Book("B1")
        book.add_child("B2")
        assert book.contains_child("B2") is True
        book.remove_child("B2")
        assert book.contains_child("B2") is False
```

**Step 2: Implement in PyBook**

Add methods to the `#[pymethods] impl PyBook` block:

```rust
fn is_root(&self) -> bool { self.inner.is_root() }
fn contains_position(&self, position_id: &str) -> bool {
    self.inner.contains_position(&PositionId::new(position_id))
}
fn contains_child(&self, child_id: &str) -> bool {
    self.inner.contains_child(&BookId::new(child_id))
}
fn add_position(&mut self, position_id: String) {
    self.inner.add_position(PositionId::new(position_id));
}
fn add_child(&mut self, child_id: String) {
    self.inner.add_child(BookId::new(child_id));
}
fn remove_position(&mut self, position_id: &str) {
    self.inner.remove_position(&PositionId::new(position_id));
}
fn remove_child(&mut self, child_id: &str) {
    self.inner.remove_child(&BookId::new(child_id));
}
```

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add Book.is_root(), contains_position/child(), add/remove methods"
```

### Task 4.3: Bind aggregate_by_multiple_attributes

**Files:**
- Modify: `finstack-py/src/portfolio/grouping.rs`
- Modify: `finstack-py/finstack/portfolio/grouping.pyi`
- Modify: `finstack-py/finstack/portfolio/__init__.pyi`

**Step 1: Write failing test**

```python
def test_aggregate_by_multiple_attributes(self):
    from finstack.portfolio import aggregate_by_multiple_attributes
    # Build portfolio with positions tagged with sector and rating
    # Aggregate by both
    result = aggregate_by_multiple_attributes(valuation, portfolio, ["sector", "rating"])
    assert isinstance(result, dict)
    # Keys should be tuples of attribute values
    for key in result:
        assert isinstance(key, tuple)
```

**Step 2: Implement binding**

```rust
#[pyfunction]
fn py_aggregate_by_multiple_attributes(
    valuation: &Bound<'_, PyAny>,
    portfolio: &Bound<'_, PyAny>,
    attribute_keys: Vec<String>,
) -> PyResult<Py<PyAny>> {
    let val = extract_portfolio_valuation(valuation)?;
    let port = extract_portfolio(portfolio)?;
    let keys: Vec<&str> = attribute_keys.iter().map(|s| s.as_str()).collect();

    let result = finstack_portfolio::grouping::aggregate_by_multiple_attributes(
        &val, &port.positions, &keys, port.base_ccy,
    ).map_err(portfolio_to_py)?;

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        for (key_vec, money) in result {
            let tuple = PyTuple::new(py, key_vec)?;
            dict.set_item(tuple, PyMoney::new(money))?;
        }
        Ok(dict.into())
    })
}
```

Register in the grouping module's export list.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): bind aggregate_by_multiple_attributes for multi-key grouping"
```

### Task 4.4: Bind PortfolioSpec/PositionSpec and to_spec/from_spec

**Files:**
- Modify: `finstack-py/src/portfolio/types.rs` (PositionSpec + Position.to_spec/from_spec)
- Modify: `finstack-py/src/portfolio/positions.rs` (PortfolioSpec + Portfolio.to_spec/from_spec)
- Modify: `finstack-py/finstack/portfolio/types.pyi`
- Modify: `finstack-py/finstack/portfolio/portfolio.pyi`

**Step 1: Write failing test**

```python
def test_position_spec_roundtrip(self):
    from finstack.portfolio import Position, PositionUnit
    from finstack.core.currency import USD
    from finstack.valuations.instruments import Equity

    equity = Equity.builder("EQ-1").ticker("TEST").currency(USD).price(100.0).build()
    pos = Position("P1", "E1", "EQ-1", equity, 100.0, PositionUnit.UNITS)
    spec = pos.to_spec()
    assert spec.position_id == "P1"
    assert spec.entity_id == "E1"

def test_portfolio_spec_roundtrip(self):
    portfolio = self._build_test_portfolio()
    spec = portfolio.to_spec()
    assert spec.id == portfolio.id
    json_str = spec.to_json()
    assert isinstance(json_str, str)
```

**Step 2: Implement PositionSpec and PortfolioSpec PyClasses**

Bind as read-only PyClasses wrapping the Rust spec types with `to_json()`/`from_json()` static methods using serde_json. Add `to_spec()` and `from_spec()` methods to `PyPosition` and `PyPortfolio`.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): bind PortfolioSpec/PositionSpec with to_spec/from_spec and JSON round-trip"
```

---

## Stream 5: Margin Module Enhancement (P1-P2)

### Task 5.1: Add NettingSetManager iterator and missing methods

**Files:**
- Modify: `finstack-py/src/portfolio/margin.rs`
- Modify: `finstack-py/finstack/portfolio/margin.pyi`

**Step 1: Write failing test**

```python
class TestNettingSetManagerParity:
    def test_iter(self):
        from finstack.portfolio import NettingSetManager, NettingSetId
        mgr = NettingSetManager()
        mgr = mgr.with_default_set(NettingSetId.bilateral("CP1", "CSA1"))
        # Should be iterable
        items = list(mgr)
        assert isinstance(items, list)

    def test_len(self):
        from finstack.portfolio import NettingSetManager, NettingSetId
        mgr = NettingSetManager()
        mgr = mgr.with_default_set(NettingSetId.bilateral("CP1", "CSA1"))
        assert len(mgr) == mgr.count()

    def test_get_or_create(self):
        from finstack.portfolio import NettingSetManager, NettingSetId
        mgr = NettingSetManager()
        nid = NettingSetId.bilateral("CP1", "CSA1")
        ns = mgr.get_or_create(nid)
        assert ns is not None
        assert mgr.count() == 1
```

**Step 2: Implement**

Add `__iter__`, `__len__`, `get_or_create`, `merge_sensitivities` to `PyNettingSetManager`.

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add NettingSetManager.__iter__/__len__, get_or_create, merge_sensitivities"
```

### Task 5.2: Add PortfolioMarginResult methods

**Step 1: Write failing test**

```python
def test_margin_result_netting_set_count(self):
    result = self._calculate_margin()
    assert hasattr(result, 'netting_set_count')
    assert result.netting_set_count() == len(list(result))

def test_margin_result_iter(self):
    result = self._calculate_margin()
    items = list(result)
    assert len(items) > 0
    for nid, margin in items:
        assert isinstance(nid, str)
```

**Step 2: Implement `netting_set_count`, `__iter__`, `__len__` on PyPortfolioMarginResult**

**Step 3: Build, test, commit**

```bash
git commit -m "feat(py): add PortfolioMarginResult.netting_set_count, __iter__, __len__"
```

### Task 5.3: Add NettingSetMargin constructor and PortfolioMarginAggregator.netting_set_count

Add `__init__` and `with_simm_breakdown` to `PyNettingSetMargin`. Add `netting_set_count()` to `PyPortfolioMarginAggregator`.

```bash
git commit -m "feat(py): add NettingSetMargin constructor and PortfolioMarginAggregator.netting_set_count"
```

---

## Final Verification

### Task 6.1: Full build and test suite

**Step 1: Clean build**

```bash
maturin develop -m finstack-py/Cargo.toml --release
```

**Step 2: Run full portfolio test suite**

```bash
pytest finstack-py/tests/parity/test_portfolio_parity.py -v --tb=short
pytest finstack-py/tests/test_margin.py -v --tb=short
```

**Step 3: Run type checker on stubs**

```bash
pyright finstack-py/finstack/portfolio/
```

**Step 4: Verify all exports in **init**.pyi are complete**

Manually verify all new types (`ApplicationReport`, `MaxYieldWithCccLimitResult`, `DefaultLpOptimizer`, `PortfolioSpec`, `PositionSpec`) are re-exported from `finstack.portfolio.__init__.pyi`.

**Step 5: Final commit**

```bash
git commit -m "chore(py): verify portfolio binding parity — all P0-P2 items complete"
```
