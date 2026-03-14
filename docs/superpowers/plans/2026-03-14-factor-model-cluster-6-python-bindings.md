# Cluster 6: Python Bindings — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the factor model API to Python via PyO3 bindings, matching the existing pattern used throughout `finstack-py`. Users should be able to construct a `FactorModelConfig` in Python, build a `FactorModel`, run analysis, and use the what-if engine — all with Pythonic ergonomics.

**Architecture:** Python bindings live in `finstack-py/src/portfolio/factor_model/`. Follow the existing pattern: one Rust file per Python class, a `register()` function that adds classes to the `finstack` Python module. Config types use Pydantic-style constructors (all fields as `__init__` params). Result types are read-only Python objects with property accessors.

**Tech Stack:** Rust, PyO3, maturin

**Spec Reference:** `docs/superpowers/specs/2026-03-14-statistical-risk-factor-model-design.md` — Section 4 (Python API)

**Depends on:** Clusters 1-5 (all Rust implementation complete)

**Context:** The existing Python bindings structure is at `finstack-py/src/lib.rs`. Modules are registered via `register()` functions. See `finstack-py/src/core/analytics/` for an example of how analytics are bound. Python types use `#[pyclass]`, `#[pymethods]`, and `#[new]` for constructors.

---

## Task 1: Bind core config types

**Files:**

- Create: `finstack-py/src/portfolio/factor_model/mod.rs`
- Create: `finstack-py/src/portfolio/factor_model/config.rs`
- Modify: `finstack-py/src/portfolio/mod.rs` — add `pub mod factor_model;`

- [ ] **Step 1: Write Python test**

Create `finstack-py/tests/test_factor_model.py`:

```python
import pytest
from finstack import (
    FactorModelConfig,
    FactorDefinition,
    FactorCovarianceMatrix,
    MatchingConfig,
    MappingRule,
    DependencyFilter,
    AttributeFilter,
    MarketMapping,
)


def test_factor_definition_construction():
    fd = FactorDefinition(
        id="USD-Rates",
        factor_type="Rates",
        market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
    )
    assert fd.id == "USD-Rates"
    assert fd.factor_type == "Rates"


def test_covariance_matrix_construction():
    cov = FactorCovarianceMatrix(
        factor_ids=["Rates", "Credit"],
        matrix=[[0.04, 0.01], [0.01, 0.09]],
    )
    assert cov.n_factors() == 2
    assert abs(cov.variance("Rates") - 0.04) < 1e-12
    assert abs(cov.correlation("Rates", "Credit") - 0.01 / (0.2 * 0.3)) < 1e-10


def test_covariance_invalid_matrix():
    with pytest.raises(Exception):
        FactorCovarianceMatrix(
            factor_ids=["Rates", "Credit"],
            matrix=[[1.0, 3.0], [3.0, 1.0]],  # not PSD
        )


def test_matching_config_mapping_table():
    config = MatchingConfig.mapping_table([
        MappingRule(
            dependency_filter=DependencyFilter(dependency_type="Hazard"),
            attribute_filter=AttributeFilter(meta=[("region", "NA")]),
            factor_id="NA-Credit",
        ),
    ])
    assert config is not None


def test_full_config_construction():
    config = FactorModelConfig(
        factors=[
            FactorDefinition(
                id="USD-Rates",
                factor_type="Rates",
                market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
            ),
        ],
        covariance=FactorCovarianceMatrix(
            factor_ids=["USD-Rates"],
            matrix=[[0.04]],
        ),
        matching=MatchingConfig.mapping_table([]),
        pricing_mode="DeltaBased",
        risk_measure="Variance",
    )
    assert len(config.factors) == 1


def test_config_json_roundtrip():
    config = FactorModelConfig(
        factors=[
            FactorDefinition(
                id="Rates",
                factor_type="Rates",
                market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
            ),
        ],
        covariance=FactorCovarianceMatrix(
            factor_ids=["Rates"],
            matrix=[[0.04]],
        ),
        matching=MatchingConfig.mapping_table([]),
        pricing_mode="DeltaBased",
        risk_measure="Variance",
    )
    json_str = config.to_json()
    back = FactorModelConfig.from_json(json_str)
    assert len(back.factors) == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd finstack-py && uv run pytest tests/test_factor_model.py -v`
Expected: FAIL — classes don't exist

- [ ] **Step 3: Implement Python bindings for config types**

Create `finstack-py/src/portfolio/factor_model/config.rs`. Follow the existing binding pattern in the codebase. Key classes:

**`PyFactorDefinition`**:

```rust
#[pyclass(name = "FactorDefinition")]
#[derive(Clone)]
pub struct PyFactorDefinition {
    inner: finstack_core::factor_model::FactorDefinition,
}

#[pymethods]
impl PyFactorDefinition {
    #[new]
    #[pyo3(signature = (id, factor_type, market_mapping, description = None))]
    fn new(
        id: String,
        factor_type: String,
        market_mapping: PyMarketMapping,
        description: Option<String>,
    ) -> PyResult<Self> {
        // Parse factor_type string → FactorType enum
        // Wrap into FactorDefinition
    }

    #[getter]
    fn id(&self) -> String { self.inner.id.as_str().to_string() }

    #[getter]
    fn factor_type(&self) -> String { /* serialize enum to string */ }
}
```

**`PyMarketMapping`** — with static constructors:

```rust
#[pyclass(name = "MarketMapping")]
pub struct PyMarketMapping { inner: MarketMapping }

#[pymethods]
impl PyMarketMapping {
    #[staticmethod]
    fn curve_parallel(curve_ids: Vec<String>, units: String) -> PyResult<Self> { /* ... */ }

    #[staticmethod]
    fn curve_bucketed(curve_id: String, tenor_weights: Vec<(f64, f64)>) -> PyResult<Self> { /* ... */ }

    #[staticmethod]
    fn equity_spot(tickers: Vec<String>) -> PyResult<Self> { /* ... */ }

    #[staticmethod]
    fn fx_rate(base: String, quote: String) -> PyResult<Self> { /* ... */ }

    #[staticmethod]
    fn vol_shift(surface_ids: Vec<String>, units: String) -> PyResult<Self> { /* ... */ }
}
```

**`PyFactorCovarianceMatrix`** — accepts nested list from Python:

```rust
#[pyclass(name = "FactorCovarianceMatrix")]
pub struct PyFactorCovarianceMatrix { inner: FactorCovarianceMatrix }

#[pymethods]
impl PyFactorCovarianceMatrix {
    #[new]
    fn new(factor_ids: Vec<String>, matrix: Vec<Vec<f64>>) -> PyResult<Self> {
        // Flatten matrix to row-major Vec<f64>
        let n = factor_ids.len();
        let data: Vec<f64> = matrix.into_iter().flatten().collect();
        let ids = factor_ids.into_iter().map(FactorId::new).collect();
        let inner = FactorCovarianceMatrix::new(ids, data)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn n_factors(&self) -> usize { self.inner.n_factors() }
    fn variance(&self, factor_id: &str) -> f64 { self.inner.variance(&FactorId::new(factor_id)) }
    fn covariance(&self, f1: &str, f2: &str) -> f64 { /* ... */ }
    fn correlation(&self, f1: &str, f2: &str) -> f64 { /* ... */ }
}
```

**`PyMatchingConfig`**, **`PyMappingRule`**, **`PyDependencyFilter`**, **`PyAttributeFilter`** — follow similar patterns.

**`PyFactorModelConfig`** — aggregates all config types:

```rust
#[pyclass(name = "FactorModelConfig")]
pub struct PyFactorModelConfig { inner: FactorModelConfig }

#[pymethods]
impl PyFactorModelConfig {
    #[new]
    #[pyo3(signature = (factors, covariance, matching, pricing_mode, risk_measure, bump_size=None, unmatched_policy=None))]
    fn new(/* params */) -> PyResult<Self> { /* ... */ }

    fn to_json(&self) -> PyResult<String> { /* serde_json::to_string_pretty */ }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> { /* serde_json::from_str */ }
}
```

- [ ] **Step 4: Create `register()` function and wire into module tree**

In `finstack-py/src/portfolio/factor_model/mod.rs`:

```rust
pub mod config;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "factor_model")?;
    m.add_class::<config::PyFactorDefinition>()?;
    m.add_class::<config::PyMarketMapping>()?;
    m.add_class::<config::PyFactorCovarianceMatrix>()?;
    m.add_class::<config::PyMatchingConfig>()?;
    m.add_class::<config::PyMappingRule>()?;
    m.add_class::<config::PyDependencyFilter>()?;
    m.add_class::<config::PyAttributeFilter>()?;
    m.add_class::<config::PyFactorModelConfig>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
```

Register from `finstack-py/src/portfolio/mod.rs`.

- [ ] **Step 5: Build and run tests**

Run: `cd finstack-py && uv run maturin develop --release && uv run pytest tests/test_factor_model.py -v`
Expected: Tests PASS

- [ ] **Step 6: Commit**

```bash
git add finstack-py/src/portfolio/factor_model/ finstack-py/tests/test_factor_model.py
git commit -m "feat(factor-model): add Python bindings for config types"
```

---

## Task 2: Bind `FactorModel` and analysis methods

**Files:**

- Create: `finstack-py/src/portfolio/factor_model/model.rs`
- Modify: `finstack-py/src/portfolio/factor_model/mod.rs`

- [ ] **Step 1: Write Python test**

Add to `finstack-py/tests/test_factor_model.py`:

```python
def test_factor_model_builder():
    config = FactorModelConfig(
        factors=[
            FactorDefinition(
                id="Rates",
                factor_type="Rates",
                market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
            ),
        ],
        covariance=FactorCovarianceMatrix(
            factor_ids=["Rates"],
            matrix=[[0.04]],
        ),
        matching=MatchingConfig.mapping_table([
            MappingRule(
                dependency_filter=DependencyFilter(dependency_type="Discount"),
                attribute_filter=AttributeFilter(),
                factor_id="Rates",
            ),
        ]),
        pricing_mode="DeltaBased",
        risk_measure="Variance",
    )

    model = FactorModelBuilder().config(config).build()
    assert model is not None
    assert len(model.factors()) == 1
```

Note: A full integration test with `model.analyze(portfolio, market, as_of)` requires constructing a Portfolio and MarketContext in Python. Look at existing Python tests (e.g., in `finstack-py/tests/` or `finstack-py/examples/`) for how these are built, and extend with a factor model analysis call.

- [ ] **Step 2: Implement model bindings**

```rust
#[pyclass(name = "FactorModelBuilder")]
pub struct PyFactorModelBuilder {
    config: Option<PyFactorModelConfig>,
}

#[pymethods]
impl PyFactorModelBuilder {
    #[new]
    fn new() -> Self { Self { config: None } }

    fn config(mut slf: PyRefMut<'_, Self>, config: PyFactorModelConfig) -> PyRefMut<'_, Self> {
        slf.config = Some(config);
        slf
    }

    fn build(&self) -> PyResult<PyFactorModel> {
        let config = self.config.as_ref()
            .ok_or_else(|| PyValueError::new_err("config is required"))?;
        let model = FactorModelBuilder::new()
            .config(config.inner.clone())
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyFactorModel { inner: Arc::new(model) })
    }
}

#[pyclass(name = "FactorModel")]
pub struct PyFactorModel {
    inner: Arc<FactorModel>,
}

#[pymethods]
impl PyFactorModel {
    fn factors(&self) -> Vec<PyFactorDefinition> { /* ... */ }

    fn analyze(
        &self,
        portfolio: &PyPortfolio,
        market: &PyMarketContext,
        as_of: PyDate,
    ) -> PyResult<PyRiskDecomposition> { /* ... */ }

    fn assign_factors(&self, portfolio: &PyPortfolio) -> PyResult<PyFactorAssignmentReport> { /* ... */ }

    fn compute_sensitivities(
        &self,
        portfolio: &PyPortfolio,
        market: &PyMarketContext,
        as_of: PyDate,
    ) -> PyResult<PySensitivityMatrix> { /* ... */ }
}
```

- [ ] **Step 3: Build and run tests**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(factor-model): add Python bindings for FactorModel and builder"
```

---

## Task 3: Bind result types and WhatIfEngine

**Files:**

- Create: `finstack-py/src/portfolio/factor_model/results.rs`
- Create: `finstack-py/src/portfolio/factor_model/whatif.rs`
- Modify: `finstack-py/src/portfolio/factor_model/mod.rs`

- [ ] **Step 1: Write Python test**

Add to `finstack-py/tests/test_factor_model.py`:

```python
def test_risk_decomposition_properties():
    # This test requires a full analysis run — use fixture
    # For now, test that the classes are importable
    from finstack import RiskDecomposition, FactorContribution, WhatIfEngine
    assert RiskDecomposition is not None


def test_what_if_stress():
    # Integration test: requires portfolio + market setup
    # Verify that factor_stress returns a StressResult
    pass  # placeholder for integration test
```

- [ ] **Step 2: Implement result type bindings**

```rust
#[pyclass(name = "RiskDecomposition")]
pub struct PyRiskDecomposition { inner: RiskDecomposition }

#[pymethods]
impl PyRiskDecomposition {
    #[getter]
    fn total_risk(&self) -> f64 { self.inner.total_risk }

    #[getter]
    fn residual_risk(&self) -> f64 { self.inner.residual_risk }

    #[getter]
    fn factor_contributions(&self) -> Vec<PyFactorContribution> {
        self.inner.factor_contributions.iter()
            .map(|c| PyFactorContribution { inner: c.clone() })
            .collect()
    }
}

#[pyclass(name = "FactorContribution")]
pub struct PyFactorContribution { inner: FactorContribution }

#[pymethods]
impl PyFactorContribution {
    #[getter]
    fn factor_id(&self) -> String { self.inner.factor_id.as_str().to_string() }

    #[getter]
    fn absolute_risk(&self) -> f64 { self.inner.absolute_risk }

    #[getter]
    fn relative_risk(&self) -> f64 { self.inner.relative_risk }

    #[getter]
    fn marginal_risk(&self) -> f64 { self.inner.marginal_risk }
}
```

- [ ] **Step 3: Implement WhatIfEngine bindings**

```rust
#[pyclass(name = "WhatIfEngine")]
pub struct PyWhatIfEngine { /* holds references via Arc */ }

#[pymethods]
impl PyWhatIfEngine {
    fn position_what_if(&self, changes: Vec<PyPositionChange>) -> PyResult<PyWhatIfResult> { /* ... */ }
    fn factor_stress(&self, stresses: Vec<(String, f64)>) -> PyResult<PyStressResult> { /* ... */ }
}
```

- [ ] **Step 4: Register all classes and build**

- [ ] **Step 5: Run full Python test suite**

Run: `cd finstack-py && uv run maturin develop --release && uv run pytest tests/test_factor_model.py -v`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(factor-model): add Python bindings for results and WhatIfEngine"
```

---

## Task 4: Add type stubs

**Files:**

- Create/Modify: `finstack-py/finstack/*.pyi` — add type stubs for all new classes

**Context:** Type stubs in this project are manually maintained (not auto-generated). Look at existing `.pyi` files for the pattern.

- [ ] **Step 1: Add stubs for all factor model classes**

```python
# In appropriate .pyi file
class FactorDefinition:
    id: str
    factor_type: str
    def __init__(self, id: str, factor_type: str, market_mapping: MarketMapping, description: str | None = None) -> None: ...

class MarketMapping:
    @staticmethod
    def curve_parallel(curve_ids: list[str], units: str) -> MarketMapping: ...
    @staticmethod
    def equity_spot(tickers: list[str]) -> MarketMapping: ...
    # ... etc

class FactorCovarianceMatrix:
    def __init__(self, factor_ids: list[str], matrix: list[list[float]]) -> None: ...
    def n_factors(self) -> int: ...
    def variance(self, factor_id: str) -> float: ...
    def covariance(self, f1: str, f2: str) -> float: ...
    def correlation(self, f1: str, f2: str) -> float: ...

class FactorModelConfig:
    factors: list[FactorDefinition]
    def __init__(self, ...) -> None: ...
    def to_json(self) -> str: ...
    @staticmethod
    def from_json(json: str) -> FactorModelConfig: ...

class FactorModelBuilder:
    def __init__(self) -> None: ...
    def config(self, config: FactorModelConfig) -> FactorModelBuilder: ...
    def build(self) -> FactorModel: ...

class FactorModel:
    def factors(self) -> list[FactorDefinition]: ...
    def analyze(self, portfolio: Portfolio, market: MarketContext, as_of: date) -> RiskDecomposition: ...
    def assign_factors(self, portfolio: Portfolio) -> FactorAssignmentReport: ...
    def compute_sensitivities(self, portfolio: Portfolio, market: MarketContext, as_of: date) -> SensitivityMatrix: ...

class RiskDecomposition:
    total_risk: float
    residual_risk: float
    factor_contributions: list[FactorContribution]

class FactorContribution:
    factor_id: str
    absolute_risk: float
    relative_risk: float
    marginal_risk: float
```

- [ ] **Step 2: Run type checking**

Run: `cd finstack-py && uv run pyright`
Expected: No errors on new stubs

- [ ] **Step 3: Commit**

```bash
git add finstack-py/finstack/
git commit -m "feat(factor-model): add Python type stubs for factor model API"
```

---

## Task 5: Write integration example

**Files:**

- Create: `finstack-py/examples/scripts/factor_model_example.py`

- [ ] **Step 1: Write a complete example**

```python
"""Factor Model: Cross-Asset Risk Decomposition Example

Demonstrates building a 3-factor model (Rates, Credit, Equity),
analyzing a mixed portfolio, and running what-if scenarios.
"""
from finstack import (
    FactorModelConfig,
    FactorModelBuilder,
    FactorDefinition,
    FactorCovarianceMatrix,
    MatchingConfig,
    MappingRule,
    DependencyFilter,
    AttributeFilter,
    MarketMapping,
    # ... portfolio and market data imports
)

# 1. Define factors
factors = [
    FactorDefinition(
        id="USD-Rates",
        factor_type="Rates",
        market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
        description="US dollar interest rates",
    ),
    FactorDefinition(
        id="NA-Credit",
        factor_type="Credit",
        market_mapping=MarketMapping.curve_parallel(["NA-IG-HAZARD"], units="bp"),
        description="North American investment grade credit",
    ),
    FactorDefinition(
        id="US-Equity",
        factor_type="Equity",
        market_mapping=MarketMapping.equity_spot(["SPX"]),
        description="US equity market",
    ),
]

# 2. Factor covariance matrix (annualized)
covariance = FactorCovarianceMatrix(
    factor_ids=["USD-Rates", "NA-Credit", "US-Equity"],
    matrix=[
        [0.0004, 0.0001, -0.0002],
        [0.0001, 0.0009,  0.0003],
        [-0.0002, 0.0003,  0.0400],
    ],
)

# 3. Matching rules
matching = MatchingConfig.mapping_table([
    MappingRule(
        dependency_filter=DependencyFilter(dependency_type="Discount"),
        attribute_filter=AttributeFilter(),
        factor_id="USD-Rates",
    ),
    MappingRule(
        dependency_filter=DependencyFilter(dependency_type="Hazard"),
        attribute_filter=AttributeFilter(meta=[("region", "NA")]),
        factor_id="NA-Credit",
    ),
    MappingRule(
        dependency_filter=DependencyFilter(dependency_type="Spot"),
        attribute_filter=AttributeFilter(),
        factor_id="US-Equity",
    ),
])

# 4. Build model
config = FactorModelConfig(
    factors=factors,
    covariance=covariance,
    matching=matching,
    pricing_mode="DeltaBased",
    risk_measure="Variance",
)

model = FactorModelBuilder().config(config).build()

# 5. Analyze portfolio
# decomposition = model.analyze(portfolio, market, as_of)
# for fc in decomposition.factor_contributions:
#     print(f"{fc.factor_id}: {fc.relative_risk:.1%} of total risk")
#
# 6. What-if: stress rates +2σ
# engine = model.what_if(decomposition, sensitivities, portfolio, market, as_of)
# stress = engine.factor_stress([("USD-Rates", 2.0)])
# print(f"P&L impact: {stress.total_pnl:,.0f}")
```

- [ ] **Step 2: Commit**

```bash
git add finstack-py/examples/scripts/factor_model_example.py
git commit -m "docs(factor-model): add Python example for factor risk decomposition"
```
