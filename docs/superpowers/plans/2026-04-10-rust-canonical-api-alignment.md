# Greenfield Rust-Canonical Binding Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current Python and WASM bindings with greenfield bindings that mirror the Rust umbrella crate and sub-crate structure exactly, with no compatibility paths and no legacy public exports.

**Architecture:** Promote `correlation` and `monte_carlo` into the Rust umbrella crate, move the tracked parity contract to repo root, build a new Python binding tree under `finstack-py/src/bindings/`, build a new WASM wrapper tree under `finstack-wasm/src/api/`, publish a namespaced JS/TS facade instead of the raw flat WASM package, disconnect legacy binding trees from public registration, then delete them once the new crate domains pass tests.

**Tech Stack:** Rust workspace crates, PyO3, wasm-bindgen, JS/TS facade files, Python stubs, pytest, wasm-bindgen-test, Node test runner, `uv`, audit scripts under `scripts/audits`, and GitHub Actions.

---

### Task 1: Lock the canonical contract and Rust umbrella exports

**Files:**
- Create: `parity_contract.toml`
- Delete: `finstack-py/parity_contract.toml`
- Modify: `finstack/Cargo.toml`
- Modify: `finstack/src/lib.rs`
- Modify: `scripts/audits/audit_topology.py`
- Modify: `finstack-py/tests/parity/test_topology_parity.py`
- Create: `finstack/tests/umbrella_exports.rs`

- [ ] **Step 1: Write the failing contract and umbrella tests**

```python
REPO_ROOT = Path(__file__).parent.parent.parent.parent
CONTRACT_PATH = REPO_ROOT / "parity_contract.toml"

def test_contract_declares_exact_top_level_domains() -> None:
    contract = _load_contract()
    expected = {
        "core",
        "analytics",
        "margin",
        "valuations",
        "statements",
        "statements_analytics",
        "portfolio",
        "scenarios",
        "correlation",
        "monte_carlo",
    }
    assert expected == set(contract["crates"])
```

```rust
#[test]
fn umbrella_reexports_all_binding_root_domains() {
    let _ = std::any::type_name::<finstack::analytics::Performance>();
    let _ = std::any::type_name::<finstack::correlation::GaussianCopula>();
    let _ = std::any::type_name::<finstack::monte_carlo::time_grid::TimeGrid>();
    let _ = std::any::type_name::<finstack::monte_carlo::rng::PhiloxRng>();
    let _ = std::any::type_name::<finstack::statements_analytics::analysis::Alignment>();
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_topology_parity.py -k exact_top_level_domains -q`

Run: `cargo test -p finstack --test umbrella_exports --features all`

Expected: FAIL because the contract still lives under `finstack-py/` and the umbrella crate does not yet re-export `correlation` and `monte_carlo`.

- [ ] **Step 3: Move the tracked contract to repo root**

Run: `mv "finstack-py/parity_contract.toml" "parity_contract.toml"`

Expected: the tracked contract now sits beside the umbrella crate.

- [ ] **Step 4: Promote `correlation` and `monte_carlo` into the umbrella crate**

```toml
[features]
correlation = ["core", "dep:finstack_correlation"]
monte_carlo = ["core", "dep:finstack_monte_carlo"]
all = [
  "core",
  "analytics",
  "margin",
  "statements",
  "valuations",
  "portfolio",
  "scenarios",
  "correlation",
  "monte_carlo",
]

[dependencies]
finstack_correlation = { package = "finstack-correlation", path = "correlation", optional = true }
finstack_monte_carlo = { package = "finstack-monte-carlo", path = "monte_carlo", optional = true }
```

```rust
#[cfg(feature = "correlation")]
pub use finstack_correlation as correlation;

#[cfg(feature = "monte_carlo")]
pub use finstack_monte_carlo as monte_carlo;
```

- [ ] **Step 5: Replace the contract schema with crate-only canonical mappings**

```toml
[meta]
version = "3.0.0"
canonical_language = "rust"
umbrella_crate = "finstack"
umbrella_lib = "finstack/src/lib.rs"

[crates.core]
rust_crate = "finstack-core"
rust_umbrella = "finstack::core"
python_package = "finstack.core"
wasm_namespace = "core"

[crates.analytics]
rust_crate = "finstack-analytics"
rust_umbrella = "finstack::analytics"
python_package = "finstack.analytics"
wasm_namespace = "analytics"

[crates.margin]
rust_crate = "finstack-margin"
rust_umbrella = "finstack::margin"
python_package = "finstack.margin"
wasm_namespace = "margin"

[crates.valuations]
rust_crate = "finstack-valuations"
rust_umbrella = "finstack::valuations"
python_package = "finstack.valuations"
wasm_namespace = "valuations"

[crates.statements]
rust_crate = "finstack-statements"
rust_umbrella = "finstack::statements"
python_package = "finstack.statements"
wasm_namespace = "statements"

[crates.statements_analytics]
rust_crate = "finstack-statements-analytics"
rust_umbrella = "finstack::statements_analytics"
python_package = "finstack.statements_analytics"
wasm_namespace = "statements_analytics"

[crates.portfolio]
rust_crate = "finstack-portfolio"
rust_umbrella = "finstack::portfolio"
python_package = "finstack.portfolio"
wasm_namespace = "portfolio"

[crates.scenarios]
rust_crate = "finstack-scenarios"
rust_umbrella = "finstack::scenarios"
python_package = "finstack.scenarios"
wasm_namespace = "scenarios"

[crates.correlation]
rust_crate = "finstack-correlation"
rust_umbrella = "finstack::correlation"
python_package = "finstack.correlation"
wasm_namespace = "correlation"

[crates.monte_carlo]
rust_crate = "finstack-monte-carlo"
rust_umbrella = "finstack::monte_carlo"
python_package = "finstack.monte_carlo"
wasm_namespace = "monte_carlo"
```

Implementation rule: do not add alias sections, compatibility metadata, or exception tables.

- [ ] **Step 6: Update topology tooling to read the repo-root contract**

```python
project_root = script_dir.parent.parent
contract_path = project_root / "parity_contract.toml"
```

- [ ] **Step 7: Re-run the tests**

Run: `uv run pytest finstack-py/tests/parity/test_topology_parity.py -k exact_top_level_domains -q`

Run: `cargo test -p finstack --test umbrella_exports --features all`

Expected: PASS.

- [ ] **Step 8: Commit the contract and umbrella changes**

```bash
git add parity_contract.toml finstack/Cargo.toml finstack/src/lib.rs scripts/audits/audit_topology.py finstack-py/tests/parity/test_topology_parity.py finstack/tests/umbrella_exports.rs
git commit -m "refactor: lock greenfield binding contract to umbrella crate"
```

### Task 2: Create the new Python public root and disconnect legacy registration

**Files:**
- Create: `finstack-py/src/bindings/mod.rs`
- Create: `finstack-py/src/bindings/core/mod.rs`
- Create: `finstack-py/src/bindings/analytics/mod.rs`
- Create: `finstack-py/src/bindings/margin/mod.rs`
- Create: `finstack-py/src/bindings/valuations/mod.rs`
- Create: `finstack-py/src/bindings/statements/mod.rs`
- Create: `finstack-py/src/bindings/statements_analytics/mod.rs`
- Create: `finstack-py/src/bindings/portfolio/mod.rs`
- Create: `finstack-py/src/bindings/scenarios/mod.rs`
- Create: `finstack-py/src/bindings/correlation/mod.rs`
- Create: `finstack-py/src/bindings/monte_carlo/mod.rs`
- Modify: `finstack-py/src/lib.rs`
- Modify: `finstack-py/finstack/__init__.py`
- Create: `finstack-py/tests/parity/test_python_root_namespaces.py`

- [ ] **Step 1: Write the failing Python root tests**

```python
import importlib

def test_python_root_exports_only_crate_domains() -> None:
    import finstack

    expected = {
        "core",
        "analytics",
        "margin",
        "valuations",
        "statements",
        "statements_analytics",
        "portfolio",
        "scenarios",
        "correlation",
        "monte_carlo",
    }
    assert expected == set(finstack.__all__)
    assert not hasattr(finstack, "Currency")
    assert not hasattr(finstack, "Money")
    assert not hasattr(finstack, "build_periods")

def test_python_root_namespaces_are_importable() -> None:
    for module_name in (
        "finstack.core",
        "finstack.analytics",
        "finstack.margin",
        "finstack.valuations",
        "finstack.statements",
        "finstack.statements_analytics",
        "finstack.portfolio",
        "finstack.scenarios",
        "finstack.correlation",
        "finstack.monte_carlo",
    ):
        assert importlib.import_module(module_name) is not None
```

- [ ] **Step 2: Run the root tests to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_python_root_namespaces.py -q`

Expected: FAIL because the current root still exports convenience names and does not expose `statements_analytics` or `monte_carlo`.

- [ ] **Step 3: Create the clean internal binding root**

```rust
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) mod analytics;
pub(crate) mod core;
pub(crate) mod correlation;
pub(crate) mod margin;
pub(crate) mod monte_carlo;
pub(crate) mod portfolio;
pub(crate) mod scenarios;
pub(crate) mod statements;
pub(crate) mod statements_analytics;
pub(crate) mod valuations;

pub(crate) fn register_root<'py>(py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {
    analytics::register(py, m)?;
    core::register(py, m)?;
    correlation::register(py, m)?;
    margin::register(py, m)?;
    monte_carlo::register(py, m)?;
    portfolio::register(py, m)?;
    scenarios::register(py, m)?;
    statements::register(py, m)?;
    statements_analytics::register(py, m)?;
    valuations::register(py, m)?;
    m.setattr(
        "__all__",
        PyList::new(
            py,
            [
                "core",
                "analytics",
                "margin",
                "valuations",
                "statements",
                "statements_analytics",
                "portfolio",
                "scenarios",
                "correlation",
                "monte_carlo",
            ],
        )?,
    )?;
    Ok(())
}
```

```rust
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "analytics")?;
    module.setattr("__all__", PyList::new(py, [] as [&str; 0])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("analytics", &module)?;
    Ok(())
}
```

Create the same minimal `register()` pattern for every top-level crate domain.

- [ ] **Step 4: Rewire `finstack-py/src/lib.rs` to use only the new binding root**

```rust
mod bindings;
mod errors;

#[pymodule]
fn finstack(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__package__", "finstack")?;
    m.setattr("__doc__", "Greenfield Python bindings for Finstack.")?;
    errors::register_exceptions(py, &m)?;
    bindings::register_root(py, &m)?;
    Ok(())
}
```

Implementation rule: remove `mod core;`, `mod valuations;`, `mod statements;`, and the old public registration flow from `lib.rs`.

- [ ] **Step 5: Rewrite the Python package root to expose only the new namespaces**

```python
import sys

from . import finstack as _finstack

__all__ = (
    "core",
    "analytics",
    "margin",
    "valuations",
    "statements",
    "statements_analytics",
    "portfolio",
    "scenarios",
    "correlation",
    "monte_carlo",
)

for _name in __all__:
    _module = getattr(_finstack, _name)
    globals()[_name] = _module
    sys.modules[f"{__name__}.{_name}"] = _module

del _finstack, _module, _name, sys
```

- [ ] **Step 6: Re-run the root tests**

Run: `uv run pytest finstack-py/tests/parity/test_python_root_namespaces.py -q`

Expected: PASS.

- [ ] **Step 7: Commit the new Python root**

```bash
git add finstack-py/src/lib.rs finstack-py/src/bindings finstack-py/finstack/__init__.py finstack-py/tests/parity/test_python_root_namespaces.py
git commit -m "refactor: replace python root with greenfield crate namespaces"
```

### Task 3: Rebuild Python foundation crates `core`, `analytics`, and `correlation`

**Files:**
- Create: `finstack-py/src/bindings/core/{mod.rs,currency.rs,dates.rs,money.rs,market_data.rs}`
- Create: `finstack-py/src/bindings/analytics/mod.rs`
- Create: `finstack-py/src/bindings/correlation/mod.rs`
- Create: `finstack-py/finstack/core/__init__.pyi`
- Create: `finstack-py/finstack/analytics/__init__.pyi`
- Create: `finstack-py/finstack/correlation/__init__.pyi`
- Create: `finstack-py/tests/parity/test_python_foundation_namespaces.py`

- [ ] **Step 1: Write the failing foundation tests**

```python
def test_core_namespace_exports_dates_money_and_currency() -> None:
    from finstack.core.currency import Currency
    from finstack.core.dates import Date
    from finstack.core.money import Money

    usd = Currency("USD")
    cash = Money(100.0, usd)
    as_of = Date(2025, 1, 15)

    assert usd.code == "USD"
    assert cash.amount == 100.0
    assert as_of.year() == 2025

def test_analytics_namespace_exports_functions() -> None:
    from finstack.analytics import sharpe

    value = sharpe([0.01, 0.02, -0.01], 0.0)
    assert isinstance(value, float)

def test_correlation_namespace_exports_types() -> None:
    from finstack.correlation import GaussianCopula

    assert GaussianCopula is not None
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_python_foundation_namespaces.py -q`

Expected: FAIL because the top-level namespace stubs are still empty.

- [ ] **Step 3: Implement the new `core` binding root with explicit submodules**

```rust
pub(crate) mod currency;
pub(crate) mod dates;
pub(crate) mod market_data;
pub(crate) mod money;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "core")?;
    currency::register(py, &module)?;
    dates::register(py, &module)?;
    market_data::register(py, &module)?;
    money::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["currency", "dates", "market_data", "money"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("core", &module)?;
    Ok(())
}
```

- [ ] **Step 4: Implement the new `analytics` and `correlation` binding roots**

```rust
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "analytics")?;
    module.add_function(wrap_pyfunction!(sharpe, &module)?)?;
    module.add_function(wrap_pyfunction!(max_drawdown, &module)?)?;
    module.setattr("__all__", PyList::new(py, ["sharpe", "max_drawdown"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("analytics", &module)?;
    Ok(())
}
```

```rust
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "correlation")?;
    module.add_class::<PyGaussianCopula>()?;
    module.add_class::<PyStudentTCopula>()?;
    module.setattr("__all__", PyList::new(py, ["GaussianCopula", "StudentTCopula"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("correlation", &module)?;
    Ok(())
}
```

- [ ] **Step 5: Add matching stubs for the new crate roots**

```python
class Currency: pass
class Money: pass
class Date: pass
```

```python
def sharpe(returns: list[float], risk_free_rate: float = 0.0) -> float: pass
```

```python
class GaussianCopula: pass
```

- [ ] **Step 6: Re-run the tests**

Run: `uv run pytest finstack-py/tests/parity/test_python_foundation_namespaces.py -q`

Expected: PASS.

- [ ] **Step 7: Commit the foundation crates**

```bash
git add finstack-py/src/bindings/core finstack-py/src/bindings/analytics finstack-py/src/bindings/correlation finstack-py/finstack/core/__init__.pyi finstack-py/finstack/analytics/__init__.pyi finstack-py/finstack/correlation/__init__.pyi finstack-py/tests/parity/test_python_foundation_namespaces.py
git commit -m "feat: rebuild python core analytics and correlation bindings"
```

### Task 4: Rebuild Python model crates `monte_carlo` and `margin`

**Files:**
- Create: `finstack-py/src/bindings/monte_carlo/{mod.rs,time_grid.rs,rng.rs,paths.rs,results.rs}`
- Create: `finstack-py/src/bindings/margin/{mod.rs,csa.rs,parameters.rs,calculator.rs,results.rs}`
- Create: `finstack-py/finstack/monte_carlo/__init__.pyi`
- Create: `finstack-py/finstack/margin/__init__.pyi`
- Create: `finstack-py/tests/parity/test_python_model_namespaces.py`

- [ ] **Step 1: Write the failing model-crate tests**

```python
import importlib

import pytest

def test_monte_carlo_namespace_exports_greenfield_surface() -> None:
    from finstack.monte_carlo import PhiloxRng, TimeGrid

    grid = TimeGrid([0.0, 0.5, 1.0])
    rng = PhiloxRng.deterministic_from_str("seed")

    assert len(grid.times) == 3
    assert rng is not None

def test_margin_namespace_exports_greenfield_surface() -> None:
    from finstack.margin import CsaSpec, VmCalculator

    csa = CsaSpec.usd_regulatory()
    calc = VmCalculator(csa)

    assert csa.id
    assert calc is not None

def test_legacy_monte_carlo_path_is_gone() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("finstack.valuations.common.monte_carlo")
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_python_model_namespaces.py -q`

Expected: FAIL because the new crate roots are still empty and the old legacy path is still indirectly present in source.

- [ ] **Step 3: Implement the new Monte Carlo binding root**

```rust
pub(crate) mod paths;
pub(crate) mod results;
pub(crate) mod rng;
pub(crate) mod time_grid;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "monte_carlo")?;
    time_grid::register(py, &module)?;
    rng::register(py, &module)?;
    paths::register(py, &module)?;
    results::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["TimeGrid", "PhiloxRng", "PathPoint", "PathDataset", "MonteCarloResult"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("monte_carlo", &module)?;
    Ok(())
}
```

- [ ] **Step 4: Implement the new margin binding root**

```rust
pub(crate) mod calculator;
pub(crate) mod csa;
pub(crate) mod parameters;
pub(crate) mod results;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "margin")?;
    csa::register(py, &module)?;
    parameters::register(py, &module)?;
    calculator::register(py, &module)?;
    results::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["CsaSpec", "VmParameters", "ImParameters", "VmCalculator", "VmResult"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("margin", &module)?;
    Ok(())
}
```

- [ ] **Step 5: Add the new stubs and remove any legacy-path tests**

```python
class TimeGrid: pass
class PhiloxRng: pass
class MonteCarloResult: pass
```

```python
class CsaSpec: pass
class VmParameters: pass
class VmCalculator: pass
class VmResult: pass
```

- [ ] **Step 6: Re-run the tests**

Run: `uv run pytest finstack-py/tests/parity/test_python_model_namespaces.py -q`

Expected: PASS.

- [ ] **Step 7: Commit the model crates**

```bash
git add finstack-py/src/bindings/monte_carlo finstack-py/src/bindings/margin finstack-py/finstack/monte_carlo/__init__.pyi finstack-py/finstack/margin/__init__.pyi finstack-py/tests/parity/test_python_model_namespaces.py
git commit -m "feat: rebuild python monte carlo and margin bindings"
```

### Task 5: Rebuild Python product crates `valuations`, `statements`, `statements_analytics`, `portfolio`, and `scenarios`

**Files:**
- Create: `finstack-py/src/bindings/valuations/mod.rs`
- Create: `finstack-py/src/bindings/statements/mod.rs`
- Create: `finstack-py/src/bindings/statements_analytics/mod.rs`
- Create: `finstack-py/src/bindings/portfolio/mod.rs`
- Create: `finstack-py/src/bindings/scenarios/mod.rs`
- Create: `finstack-py/finstack/valuations/__init__.pyi`
- Create: `finstack-py/finstack/statements/__init__.pyi`
- Create: `finstack-py/finstack/statements_analytics/__init__.pyi`
- Create: `finstack-py/finstack/portfolio/__init__.pyi`
- Create: `finstack-py/finstack/scenarios/__init__.pyi`
- Create: `finstack-py/tests/parity/test_python_product_namespaces.py`

- [ ] **Step 1: Write the failing product-crate tests**

```python
import importlib

import pytest

def test_product_crate_namespaces_are_canonical() -> None:
    from finstack.portfolio import Portfolio
    from finstack.scenarios import ScenarioEngine
    from finstack.statements.builder import ModelBuilder
    from finstack.statements_analytics.analysis.credit.covenants import forecast_covenant
    from finstack.valuations.instruments import Bond

    assert Portfolio is not None
    assert ScenarioEngine is not None
    assert ModelBuilder is not None
    assert forecast_covenant is not None
    assert Bond is not None

def test_folded_legacy_statements_analytics_path_is_gone() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("finstack.statements.analysis")
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_python_product_namespaces.py -q`

Expected: FAIL because the new product crate roots are still empty and `statements_analytics` is not yet first-class.

- [ ] **Step 3: Implement the new product crate binding roots**

```rust
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "valuations")?;
    instruments::register(py, &module)?;
    calibration::register(py, &module)?;
    margin::register(py, &module)?;
    xva::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["instruments", "calibration", "margin", "xva"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("valuations", &module)?;
    Ok(())
}
```

```rust
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "statements_analytics")?;
    analysis::register(py, &module)?;
    extensions::register(py, &module)?;
    templates::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["analysis", "extensions", "templates"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("statements_analytics", &module)?;
    Ok(())
}
```

- [ ] **Step 4: Add the new top-level stubs and product module stubs**

```python
class Portfolio: pass
class ScenarioEngine: pass
class ModelBuilder: pass
class Bond: pass
```

```python
def forecast_covenant(*args, **kwargs): pass
```

- [ ] **Step 5: Re-run the tests**

Run: `uv run pytest finstack-py/tests/parity/test_python_product_namespaces.py -q`

Expected: PASS.

- [ ] **Step 6: Commit the product crates**

```bash
git add finstack-py/src/bindings/valuations finstack-py/src/bindings/statements finstack-py/src/bindings/statements_analytics finstack-py/src/bindings/portfolio finstack-py/src/bindings/scenarios finstack-py/finstack/valuations/__init__.pyi finstack-py/finstack/statements/__init__.pyi finstack-py/finstack/statements_analytics/__init__.pyi finstack-py/finstack/portfolio/__init__.pyi finstack-py/finstack/scenarios/__init__.pyi finstack-py/tests/parity/test_python_product_namespaces.py
git commit -m "feat: rebuild python product crate bindings"
```

### Task 6: Create the new WASM facade root and disconnect the legacy flat package

**Files:**
- Create: `finstack-wasm/src/api/mod.rs`
- Create: `finstack-wasm/src/api/{core,analytics,margin,valuations,statements,statements_analytics,portfolio,scenarios,correlation,monte_carlo}/mod.rs`
- Modify: `finstack-wasm/src/lib.rs`
- Create: `finstack-wasm/index.js`
- Create: `finstack-wasm/index.d.ts`
- Create: `finstack-wasm/exports/{core,analytics,margin,valuations,statements,statements_analytics,portfolio,scenarios,correlation,monte_carlo}.js`
- Modify: `finstack-wasm/package.json`
- Create: `finstack-wasm/tests/facade.test.mjs`

- [ ] **Step 1: Write the failing facade-root test**

```javascript
import test from "node:test";
import assert from "node:assert/strict";

import init, {
  analytics,
  core,
  correlation,
  margin,
  monte_carlo,
  portfolio,
  scenarios,
  statements,
  statements_analytics,
  valuations,
} from "../index.js";

await init();

test("facade exports crate namespaces only", () => {
  assert.equal(typeof core, "object");
  assert.equal(typeof analytics, "object");
  assert.equal(typeof correlation, "object");
  assert.equal(typeof margin, "object");
  assert.equal(typeof monte_carlo, "object");
  assert.equal(typeof valuations, "object");
  assert.equal(typeof statements, "object");
  assert.equal(typeof statements_analytics, "object");
  assert.equal(typeof portfolio, "object");
  assert.equal(typeof scenarios, "object");
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd finstack-wasm && node --test tests/facade.test.mjs`

Expected: FAIL because the package still points to the raw flat wasm-bindgen output and `index.js` does not exist.

- [ ] **Step 3: Create the new Rust-side API root**

```rust
pub mod analytics;
pub mod correlation;
pub mod margin;
pub mod monte_carlo;
pub mod portfolio;
pub mod scenarios;
pub mod statements;
pub mod statements_analytics;
pub mod valuations;

#[path = "core_ns/mod.rs"]
pub mod core_ns;
```

Implementation note: the `core` namespace module is named `core_ns` on disk to avoid shadowing Rust's `core` prelude. wasm-bindgen discovers `#[wasm_bindgen]` items by traversal, so `pub use` glob re-exports are unnecessary and must not be added -- `pub use core::*` would shadow `std::core` and break compilation.

- [ ] **Step 4: Rewire `src/lib.rs` to export only the new API tree**

```rust
mod api;

pub use api::*;
```

Implementation rule: stop publicly reexporting the old crate wrapper modules from `src/lib.rs`.

- [ ] **Step 5: Create the JS facade and switch the package entrypoint**

```javascript
import init from "./pkg/finstack_wasm.js";

export { core } from "./exports/core.js";
export { analytics } from "./exports/analytics.js";
export { margin } from "./exports/margin.js";
export { valuations } from "./exports/valuations.js";
export { statements } from "./exports/statements.js";
export { statements_analytics } from "./exports/statements_analytics.js";
export { portfolio } from "./exports/portfolio.js";
export { scenarios } from "./exports/scenarios.js";
export { correlation } from "./exports/correlation.js";
export { monte_carlo } from "./exports/monte_carlo.js";

export default init;
```

```json
{
  "main": "./index.js",
  "types": "./index.d.ts",
  "exports": {
    ".": {
      "types": "./index.d.ts",
      "default": "./index.js"
    },
    "./package.json": "./package.json"
  }
}
```

- [ ] **Step 6: Re-run the facade-root test**

Run: `cd finstack-wasm && node --test tests/facade.test.mjs`

Expected: PASS.

- [ ] **Step 7: Commit the new WASM root**

```bash
git add finstack-wasm/src/lib.rs finstack-wasm/src/api finstack-wasm/index.js finstack-wasm/index.d.ts finstack-wasm/exports finstack-wasm/package.json finstack-wasm/tests/facade.test.mjs
git commit -m "refactor: replace flat wasm package with crate facade"
```

### Task 7: Rebuild WASM foundation crates `core`, `analytics`, and `correlation`

**Files:**
- Create: `finstack-wasm/src/api/core/{mod.rs,currency.rs,dates.rs,money.rs,market_data.rs}`
- Create: `finstack-wasm/src/api/analytics/mod.rs`
- Create: `finstack-wasm/src/api/correlation/mod.rs`
- Modify: `finstack-wasm/exports/{core,analytics,correlation}.js`
- Modify: `finstack-wasm/index.d.ts`
- Create: `finstack-wasm/tests/foundation-facade.test.mjs`

- [ ] **Step 1: Write the failing foundation facade tests**

```javascript
import test from "node:test";
import assert from "node:assert/strict";

import init, { analytics, core, correlation } from "../index.js";

await init();

test("core namespace exposes Money and FsDate", () => {
  assert.equal(typeof core.Currency, "function");
  assert.equal(typeof core.Money, "function");
  assert.equal(typeof core.dates.FsDate, "function");
});

test("analytics namespace exposes functions", () => {
  assert.equal(typeof analytics.sharpe, "function");
});

test("correlation namespace exposes types", () => {
  assert.equal(typeof correlation.GaussianCopula, "function");
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd finstack-wasm && node --test tests/foundation-facade.test.mjs`

Expected: FAIL because the namespaces are still empty.

- [ ] **Step 3: Implement the foundation namespaces in `src/api`**

File: `src/api/core_ns/mod.rs`

```rust
pub mod currency;
pub mod dates;
pub mod market_data;
pub mod money;
```

Each submodule (e.g. `currency.rs`) defines its own `#[wasm_bindgen]` wrapper types. wasm-bindgen discovers them automatically by traversal -- no `pub use` re-exports needed.

- [ ] **Step 4: Populate the JS facade namespace files**

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const core = {
  Currency: raw.Currency,
  Money: raw.Money,
  dates: {
    FsDate: raw.FsDate,
    DayCount: raw.DayCount,
    buildPeriods: raw.buildPeriods,
  },
  market_data: {
    MarketContext: raw.MarketContext,
    DiscountCurve: raw.DiscountCurve,
  },
};
```

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const analytics = {
  sharpe: raw.sharpe,
  maxDrawdown: raw.maxDrawdown,
};
```

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const correlation = {
  GaussianCopula: raw.GaussianCopula,
  StudentTCopula: raw.StudentTCopula,
};
```

- [ ] **Step 5: Re-run the tests**

Run: `cd finstack-wasm && node --test tests/foundation-facade.test.mjs`

Expected: PASS.

- [ ] **Step 6: Commit the WASM foundation crates**

```bash
git add finstack-wasm/src/api/core finstack-wasm/src/api/analytics finstack-wasm/src/api/correlation finstack-wasm/exports/core.js finstack-wasm/exports/analytics.js finstack-wasm/exports/correlation.js finstack-wasm/index.d.ts finstack-wasm/tests/foundation-facade.test.mjs
git commit -m "feat: rebuild wasm core analytics and correlation namespaces"
```

### Task 8: Rebuild WASM model and product crates

**Files:**
- Create: `finstack-wasm/src/api/monte_carlo/mod.rs`
- Create: `finstack-wasm/src/api/margin/mod.rs`
- Create: `finstack-wasm/src/api/valuations/mod.rs`
- Create: `finstack-wasm/src/api/statements/mod.rs`
- Create: `finstack-wasm/src/api/statements_analytics/mod.rs`
- Create: `finstack-wasm/src/api/portfolio/mod.rs`
- Create: `finstack-wasm/src/api/scenarios/mod.rs`
- Modify: `finstack-wasm/exports/{monte_carlo,margin,valuations,statements,statements_analytics,portfolio,scenarios}.js`
- Modify: `finstack-wasm/index.d.ts`
- Create: `finstack-wasm/tests/product-facade.test.mjs`

- [ ] **Step 1: Write the failing product facade tests**

```javascript
import test from "node:test";
import assert from "node:assert/strict";

import init, {
  margin,
  monte_carlo,
  portfolio,
  scenarios,
  statements,
  statements_analytics,
  valuations,
} from "../index.js";

await init();

test("monte carlo namespace exposes TimeGrid", () => {
  assert.equal(typeof monte_carlo.TimeGrid, "function");
});

test("margin namespace exposes CsaSpec", () => {
  assert.equal(typeof margin.CsaSpec, "function");
});

test("valuations namespace exposes Bond", () => {
  assert.equal(typeof valuations.instruments.Bond, "function");
});

test("statements analytics namespace exposes forecastCovenant", () => {
  assert.equal(typeof statements_analytics.analysis.credit.covenants.forecastCovenant, "function");
});

test("portfolio and scenarios namespaces expose engines", () => {
  assert.equal(typeof portfolio.Portfolio, "function");
  assert.equal(typeof scenarios.ScenarioEngine, "function");
  assert.equal(typeof statements.builder.ModelBuilder, "function");
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd finstack-wasm && node --test tests/product-facade.test.mjs`

Expected: FAIL because the product namespaces are not populated yet.

- [ ] **Step 3: Implement the new model and product namespace wrappers**

Implementation note: each `api/<crate>/mod.rs` should contain `#[wasm_bindgen]` wrapper types that delegate to the Rust crate. They do NOT re-export from each other. wasm-bindgen discovers all annotated items by traversal. Example wrapper pattern:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TimeGrid {
    inner: finstack_monte_carlo::time_grid::TimeGrid,
}

#[wasm_bindgen]
impl TimeGrid {
    #[wasm_bindgen(constructor)]
    pub fn new(times: Vec<f64>) -> Result<TimeGrid, JsValue> {
        finstack_monte_carlo::time_grid::TimeGrid::new(times)
            .map(|inner| TimeGrid { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

- [ ] **Step 4: Populate the remaining facade namespace files**

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const monte_carlo = {
  TimeGrid: raw.TimeGrid,
  PhiloxRng: raw.PhiloxRng,
};
```

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const valuations = {
  instruments: {
    Bond: raw.Bond,
  },
  calibration: {
    CalibrationConfig: raw.CalibrationConfig,
  },
};
```

```javascript
import * as raw from "../pkg/finstack_wasm.js";

export const statements_analytics = {
  analysis: {
    credit: {
      covenants: {
        forecastCovenant: raw.forecastCovenant,
      },
    },
  },
};
```

- [ ] **Step 5: Re-run the tests**

Run: `cd finstack-wasm && node --test tests/product-facade.test.mjs`

Expected: PASS.

- [ ] **Step 6: Commit the WASM model and product crates**

```bash
git add finstack-wasm/src/api/monte_carlo finstack-wasm/src/api/margin finstack-wasm/src/api/valuations finstack-wasm/src/api/statements finstack-wasm/src/api/statements_analytics finstack-wasm/src/api/portfolio finstack-wasm/src/api/scenarios finstack-wasm/exports/monte_carlo.js finstack-wasm/exports/margin.js finstack-wasm/exports/valuations.js finstack-wasm/exports/statements.js finstack-wasm/exports/statements_analytics.js finstack-wasm/exports/portfolio.js finstack-wasm/exports/scenarios.js finstack-wasm/index.d.ts finstack-wasm/tests/product-facade.test.mjs
git commit -m "feat: rebuild wasm model and product namespaces"
```

### Task 9: Delete legacy binding trees, rewrite parity tooling, and add the greenfield parity gate

**Files:**
- Delete: `finstack-py/src/core`
- Delete: `finstack-py/src/correlation`
- Delete: `finstack-py/src/portfolio`
- Delete: `finstack-py/src/scenarios`
- Delete: `finstack-py/src/statements`
- Delete: `finstack-py/src/valuations`
- Delete: `finstack-py/finstack/_binding_exports.py`
- Delete: `finstack-wasm/src/core`
- Delete: `finstack-wasm/src/correlation`
- Delete: `finstack-wasm/src/portfolio`
- Delete: `finstack-wasm/src/scenarios`
- Delete: `finstack-wasm/src/statements`
- Delete: `finstack-wasm/src/valuations`
- Modify: `scripts/audits/generate_parity_manifest.py`
- Modify: `scripts/audits/audit_python_api.py`
- Modify: `scripts/audits/audit_wasm_api.py`
- Modify: `scripts/audits/compare_apis.py`
- Modify: `Makefile`
- Modify: `.github/workflows/build.yml`
- Modify: `finstack-py/tests/test_parity_golden.py`
- Modify: `finstack-wasm/package.json`
- Modify: `finstack/monte_carlo/README.md`
- Modify: `finstack/margin/README.md`
- Modify: `finstack/analytics/README.md`

- [ ] **Step 1: Write the failing cleanup and gate tests**

```python
import importlib

import pytest

def test_legacy_binding_paths_are_not_importable() -> None:
    for legacy_path in (
        "finstack.valuations.common.monte_carlo",
        "finstack.statements.analysis",
        "finstack.core.analytics",
    ):
        with pytest.raises(ModuleNotFoundError):
            importlib.import_module(legacy_path)
```

```make
.PHONY: parity-check
parity-check:
	@false
```

- [ ] **Step 2: Run the tests and parity target to verify they fail**

Run: `uv run pytest finstack-py/tests/parity/test_python_model_namespaces.py finstack-py/tests/parity/test_python_product_namespaces.py -q`

Run: `make parity-check`

Expected: FAIL because the old source trees still exist and the parity target is not implemented.

- [ ] **Step 3: Delete the disconnected legacy binding trees**

Run: `rm -rf "finstack-py/src/core" "finstack-py/src/correlation" "finstack-py/src/portfolio" "finstack-py/src/scenarios" "finstack-py/src/statements" "finstack-py/src/valuations" "finstack-py/finstack/_binding_exports.py"`

Run: `rm -rf "finstack-wasm/src/core" "finstack-wasm/src/correlation" "finstack-wasm/src/portfolio" "finstack-wasm/src/scenarios" "finstack-wasm/src/statements" "finstack-wasm/src/valuations"`

Expected: only the new `finstack-py/src/bindings` and `finstack-wasm/src/api` trees remain as active binding implementations.

- [ ] **Step 4: Rewrite the audit pipeline around the greenfield contract**

```python
manifest = {
    "contract_version": contract["meta"]["version"],
    "binding_style": "greenfield",
    "python_packages": {},
    "wasm_namespaces": {},
}
```

```python
return {
    "topology": topology_findings,
    "symbols": symbol_findings,
    "methods": method_findings,
    "behavior": behavior_findings,
}
```

Implementation rule: remove any code paths that classify aliases or compatibility exports.

- [ ] **Step 5: Add the real parity gate to local workflow and CI**

```make
.PHONY: parity-check
parity-check:
	@$(call py_run,python scripts/audits/audit_topology.py --symbols)
	@$(call py_run,python scripts/audits/generate_parity_manifest.py)
	@$(call py_run,python scripts/audits/audit_python_api.py)
	@$(call py_run,python scripts/audits/audit_wasm_api.py)
	@$(call py_run,python scripts/audits/compare_apis.py)
	@$(call py_run,pytest finstack-py/tests/parity -q)
	@cd finstack-wasm && node --test tests/*.mjs
```

```yaml
  parity:
    name: Parity
    runs-on: ubuntu-latest
    needs: [test-python, test-wasm]
    steps:
      - name: Checkout repository
        uses: actions/checkout@v5

      - name: Set up Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version-file: '.python-version'

      - name: Install uv
        uses: astral-sh/setup-uv@v4
        with:
          version: "latest"

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version-file: '.nvmrc'

      - name: Run parity gate
        run: make parity-check
```

- [ ] **Step 6: Update golden tests and docs to the new crate-shaped API**

```python
from finstack.analytics import sharpe
from finstack.margin import CsaSpec, VmCalculator
from finstack.monte_carlo import PhiloxRng, TimeGrid
from finstack.statements_analytics.analysis.credit.covenants import forecast_covenant
from finstack.valuations.instruments import Bond
```

```javascript
import init, { analytics, core, correlation, margin, monte_carlo, portfolio, scenarios, statements, statements_analytics, valuations } from "finstack-wasm";
```

- [ ] **Step 7: Re-run the full greenfield verification set**

Run: `uv run pytest finstack-py/tests/parity -q`

Run: `uv run pytest finstack-py/tests/test_parity_golden.py -q`

Run: `cd finstack-wasm && node --test tests/*.mjs`

Run: `cargo test -p finstack --test umbrella_exports --features all`

Run: `make parity-check`

Expected: PASS.

- [ ] **Step 8: Commit the cleanup and parity gate**

```bash
git add parity_contract.toml scripts/audits/generate_parity_manifest.py scripts/audits/audit_python_api.py scripts/audits/audit_wasm_api.py scripts/audits/compare_apis.py Makefile .github/workflows/build.yml finstack-py/tests finstack-wasm/tests finstack/monte_carlo/README.md finstack/margin/README.md finstack/analytics/README.md
git add -A finstack-py/src finstack-wasm/src finstack-py/finstack finstack-wasm/index.js finstack-wasm/index.d.ts finstack-wasm/exports finstack-wasm/package.json
git commit -m "refactor: delete legacy bindings and enforce greenfield parity"
```

## Self-Review Checklist

- [ ] The plan contains no compatibility paths, alias exports, or forwarding modules.
- [ ] `statements_analytics` is treated as a first-class top-level binding domain.
- [ ] `margin` and `valuations.margin` are allowed to coexist when Rust exposes both.
- [ ] The Python rewrite uses `finstack-py/src/bindings/` and no longer depends on the old module tree.
- [ ] The WASM rewrite uses `finstack-wasm/src/api/` plus a JS/TS facade and no longer publishes the flat raw package as the public API.
- [ ] The old binding trees are deleted only after the new roots and crate domains exist.
- [ ] The parity tooling is contract-first and does not classify aliases because aliases are out of scope.
- [ ] Every task includes exact files, explicit commands, and no `TODO`, `TBD`, or "handle appropriately" placeholders.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-10-rust-canonical-api-alignment.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
