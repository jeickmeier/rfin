# Bindings Crates — Technical Design

**Version:** 3.0 (aligned with overall)  
**Status:** Design complete  
**Audience:** Library authors, maintainers, and advanced integrators (Python/WASM)

---

## 1) Executive Summary

The bindings crates (`rfin-python` and `rfin-wasm`) expose the Finstack library to Python and JavaScript/TypeScript with idiomatic, user‑friendly APIs that still allow access to the full depth of the Rust crates. They strictly follow a reuse‑first policy: bindings orchestrate and surface existing functionality from `core`, `statements`, `valuations`, `scenarios`, and `portfolio`; they do not implement new algorithms.

### 1.1 Key Objectives

* **Zero-cost abstractions** - Minimal overhead over core Rust functionality
* **Idiomatic APIs** - Natural feel for Python and JavaScript developers
* **Complete serde coverage** - 100% wire format stability as per overall.md §0.1
* **Cross-platform support** - Python wheels for major platforms, WASM for browsers and Node.js
* **Comprehensive documentation** - Language-specific docs with examples

### 1.2 Architectural Position

Per `overall.md` §1 and §9, the bindings are distinct subcrates and re‑use public APIs from the Rust crates:

```
Workspace
├── core          ← Foundation types, expression engine, FX
├── statements    ← Financial statements modeling
├── valuations    ← Instrument pricing and risk
├── analysis      ← Plugin analyzers
├── scenarios     ← Scenario DSL and engine
├── portfolio     ← Portfolio aggregation
├── io            ← Polars↔Arrow/CSV/Parquet interchange
├── rfin-python   ← PyO3 bindings (this document)
└── rfin-wasm     ← wasm-bindgen bindings (this document)
```

Bindings depend on the Rust crates and selectively expose functionality via feature flags. They provide high‑level façades for common workflows, and advanced users can access lower‑level objects that map closely to the Rust types.

---

## 2) Python Bindings (`rfin-python`)

### 2.1 Technology Stack

* **PyO3** - Rust-Python interoperability framework
* **maturin** - Build and publish Python wheels
* **pydantic v2** - Python data validation (mirrors serde shapes)

### 2.2 Module Architecture (public Python package layout)

```python
rfin/  
├── __init__.py         # Top-level exports
├── currency.py         # Currency helpers (thin wrappers)
├── money.py            # Money helpers (thin wrappers)
├── dates/              # Periods, calendars, daycount (from core)
│   ├── __init__.py
│   ├── calendar.py
│   ├── daycount.py
│   └── schedule.py
├── cashflow/           # Cashflow types (reused via valuations)
│   ├── __init__.py
│   └── legs.py
├── market_data/        # Market data (reused via valuations)
│   ├── __init__.py
│   ├── curves.py
│   └── surfaces.py
├── statements/         # [feature-gated] façades that call finstack-statements
├── valuations/         # [feature-gated] façades that call finstack-valuations
├── scenarios/          # [feature-gated] DSL parse/apply via finstack-scenarios
└── portfolio/          # [feature-gated] builder/runner via finstack-portfolio
```

### 2.3 Reuse‑First Wrapping Strategy

- Prefer returning/accepting simple Pythonic types (dict, list, Decimal, `datetime.date`) that mirror serde shapes where possible.
- When a Rust type needs an object wrapper, keep it thin and delegate to the underlying crate. Do not duplicate logic in Python.
- Keep constructors simple and provide classmethods for advanced cases.

```rust
/// Python wrapper for core::Currency (thin, reuse-only)
#[pyclass(name = "Currency", module = "rfin.currency")]
#[derive(Clone)]
pub struct PyCurrency {
    inner: core::Currency,
}

#[pymethods]
impl PyCurrency {
    #[new]
    fn new(code: String) -> PyResult<Self> {
        core::Currency::from_str(&code)
            .map(|inner| PyCurrency { inner })
            .map_err(convert_error)
    }
    
    #[getter]
    fn code(&self) -> String {
        self.inner.to_string()
    }
}
```

### 2.4 Error Mapping

Map Rust `FinstackError` and crate‑specific errors to friendly Python exceptions while preserving details (per `overall.md` §12):

```rust
fn convert_error(err: FinstackError) -> PyErr {
    match err {
        FinstackError::PeriodParse(e) => {
            PyErr::new::<PyValueError, _>(format!("Period parse error: {}", e))
        }
        FinstackError::CurrencyMismatch { expected, actual } => {
            PyErr::new::<PyValueError, _>(format!(
                "Currency mismatch: expected {}, got {}", expected, actual
            ))
        }
        FinstackError::Dag(e) => {
            PyErr::new::<PyRuntimeError, _>(format!("DAG error: {}", e))
        }
        _ => PyErr::new::<PyRuntimeError, _>(format!("Operation failed: {}", err))
    }
}
```

### 2.5 Core Integration (via reuse)

#### Currency and Money (core §2.1)

```python
from rfin import Currency, Money

# Currency-safe arithmetic enforced
usd_100 = Money(100.0, Currency.USD)
eur_75 = Money(75.0, Currency.EUR)

# This raises ValueError (currency mismatch)
try:
    total = usd_100 + eur_75  
except ValueError as e:
    print(e)  # "Currency mismatch: expected USD, got EUR"

# Explicit FX conversion required
fx_provider = FxMatrix(...)
eur_in_usd = eur_75.convert_to(Currency.USD, fx_provider)
total = usd_100 + eur_in_usd  # OK
```

#### Expression Engine (core §2.3)

```python
from rfin.statements import Expression, ExpressionContext

# Python wrapper exposes core's expression engine
expr = Expression.parse("revenue * (1 + growth_rate)")
context = ExpressionContext({
    "revenue": 1000000,
    "growth_rate": 0.05
})
result = expr.evaluate(context)  # 1050000
```

#### Period System (core §2.5)

```python
from rfin.dates import PeriodPlan

# Uses core's period parser
plan = PeriodPlan.build("2025Q1..2026Q4", actuals="2025Q1..Q2")
for period in plan.all:
    print(f"{period.id}: {period.start} to {period.end} (actual={period.is_actual})")
```

### 2.6 Feature‑Gated Modules

Aligned with `overall.md` §1.1 meta‑crate features; the Python wheel enables modules behind flags so users only import what they need:

```toml
[features]
default = ["core"]
statements = ["core", "dep:finstack-statements"]
valuations = ["core", "dep:finstack-valuations"]
analysis = ["statements", "valuations", "dep:finstack-analysis"]
scenarios = ["statements", "valuations", "dep:finstack-scenarios"]
portfolio = ["statements", "valuations", "scenarios", "dep:finstack-portfolio"]
all = ["statements", "valuations", "analysis", "scenarios", "portfolio"]
```

When features are enabled, additional Python modules become available and call directly into the corresponding Rust crates before adding any new convenience API:

```python
# With statements feature
from rfin.statements import FinancialModel, Node, NodeType

model = FinancialModel.builder("Acme Corp") \
    .periods("2025Q1..2026Q4", actuals="2025Q1..Q2") \
    .compute("gross_margin", "gross_profit / revenue") \
    .register_metrics("fin.basic") \
    .build()

# With valuations feature  
from rfin.valuations import InterestRateSwap, MarketData

swap = InterestRateSwap(...)
market = MarketData(as_of="2025-01-01", ...)
result = swap.price(market)

# With portfolio feature
from rfin.portfolio import Portfolio, Position

portfolio = Portfolio.builder("Fund A") \
    .plan(Currency.USD, "2025-01-01", periods) \
    .entity("OpCo", entity_refs) \
    .position(Position(...)) \
    .build()

### 2.7 High‑Level, User‑Friendly Facades (compose existing crates)

Provide ergonomic façades that orchestrate existing crates without re‑implementing functionality:

```python
from rfin import PortfolioRunner, Scenario

# Evaluate a portfolio with an optional scenario
runner = PortfolioRunner(parallel=True)
out = runner.run(portfolio, market_data, scenario=Scenario.parse("""
    market.fx.USD/EUR:+%2
    portfolio.positions."TLB-1".quantity:+%10
"""))

print(out.valuation.portfolio_total_base)
```
```

### 2.8 Performance Optimizations

#### GIL Release (heavy compute, reused engines)

Heavy computations release the GIL as per overall.md §9.1:

```rust
#[pymethods]
impl PyFinancialModel {
    fn evaluate(&self, py: Python) -> PyResult<PyResults> {
        py.allow_threads(|| {
            // Long-running evaluation without GIL
            self.inner.evaluate()
                .map(|r| PyResults::from(r))
                .map_err(convert_error)
        })
    }
}
```

#### Zero‑Copy Data Transfer

For large datasets, use NumPy arrays or Arrow:

```rust
use numpy::{PyArray1, PyArray2};

#[pymethods]
impl PyCurve {
    fn get_pillars<'py>(&self, py: Python<'py>) -> &'py PyArray1<f64> {
        PyArray1::from_slice(py, &self.inner.pillars)
    }
}
```

### 2.9 Pydantic Integration (serde parity)

Mirror serde shapes for round-trip serialization:

```python
from pydantic import BaseModel
from typing import Optional, List
from decimal import Decimal

class NodeModel(BaseModel):
    """Pydantic model matching statements::Node"""
    node_id: str
    name: Optional[str] = None
    values: Optional[dict[str, Decimal]] = None
    formula: Optional[str] = None
    node_type: str  # "Value" | "Calculated" | "Mixed"
    
    class Config:
        # Match serde behavior
        extra = "forbid"  # deny_unknown_fields
```

---

## 3) WASM Bindings (`rfin-wasm`)

### 3.1 Technology Stack

* **wasm-bindgen** - Rust-WASM interoperability
* **serde-wasm-bindgen** - Complex object serialization
* **wasm-pack** - Build tool for WASM packages

### 3.2 Module Structure

```
rfin-wasm/
├── pkg/           # Web target
├── pkg-node/      # Node.js target  
└── pkg-bundler/   # Bundler target (webpack, etc.)
```

JavaScript module exports (tree‑shakeable via features):

```javascript
// ESM imports
import { Currency, Money, Date, DayCount } from 'rfin-wasm';

// CommonJS (Node.js)
const { Currency, Money, Date, DayCount } = require('rfin-wasm');
```

### 3.3 Reuse‑First Type Wrapping Strategy

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Currency {
    inner: core::Currency,
}

#[wasm_bindgen]
impl Currency {
    #[wasm_bindgen(constructor)]
    pub fn new(code: String) -> Result<Currency, JsValue> {
        core::Currency::from_str(&code)
            .map(|inner| Currency { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.inner.to_string()
    }
    
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.code()
    }
}
```

### 3.4 Error Handling

JavaScript‑friendly error messages with preserved context:

```rust
fn convert_error(err: FinstackError) -> JsValue {
    let message = match err {
        FinstackError::PeriodParse(e) => 
            format!("Invalid period format: {}", e),
        FinstackError::CurrencyMismatch { expected, actual } =>
            format!("Currency mismatch: expected {}, got {}", expected, actual),
        _ => format!("Error: {}", err)
    };
    JsValue::from_str(&message)
}
```

### 3.5 Core Integration (via reuse)

#### Amount Arithmetic (core §2.1)

```javascript
const usd100 = new Money(100, Currency.USD);
const eur75 = new Money(75, Currency.EUR);

// Currency safety enforced
try {
    const total = usd100.add(eur75);
} catch (e) {
    console.error(e); // "Currency mismatch: expected USD, got EUR"
}

// Explicit conversion required
const eurInUsd = eur75.convertTo(Currency.USD, fxProvider);
const total = usd100.add(eurInUsd); // OK
```

#### Calendar/Daycount (core §2.2)

```javascript
import { Date, Calendar, BusDayConvention } from 'rfin-wasm';

const date = new Date(2025, 1, 15);
const calendar = Calendar.Target();
const nextBusDay = calendar.adjust(date, BusDayConvention.Following);
```

#### Period Generation (core §2.5)

```javascript
import { PeriodPlan } from 'rfin-wasm';

const plan = PeriodPlan.build("2025Q1..2026Q4", "2025Q1..Q2");
plan.periods.forEach(period => {
    console.log(`${period.id}: ${period.start} to ${period.end}`);
});
```

### 3.6 Complex Type Serialization

For complex objects, use serde-wasm-bindgen:

```rust
use serde_wasm_bindgen::{to_value, from_value};

#[wasm_bindgen]
impl FinancialModel {
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(value: JsValue) -> Result<FinancialModel, JsValue> {
        from_value(value)
            .map(|inner| FinancialModel { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

### 3.7 Feature Flags and Tree Shaking

Support selective compilation for smaller bundles:

```toml
[features]
default = ["core"]
statements = ["dep:finstack-statements"]
valuations = ["dep:finstack-valuations"]
scenarios = ["dep:finstack-scenarios"]
portfolio = ["dep:finstack-portfolio"]

# Size optimization
wee_alloc = ["dep:wee_alloc"]
```

Build with specific features:

```bash
wasm-pack build --features statements,valuations
```

### 3.8 Memory Management

Minimize allocations across the JS-WASM boundary:

```rust
// Good: Batch operations
#[wasm_bindgen]
pub fn calculate_multiple(values: &[f64]) -> Vec<f64> {
    values.iter().map(|&v| self.calculate(v)).collect()
}

// Avoid: Individual calls from JavaScript
for (let value of values) {
    results.push(calculator.calculate(value)); // Many boundary crossings
}
```

---

## 4) Shared Design Principles

### 4.1 Wire Format Stability

As per overall.md §0.1 and §12:

* All public types have 100% serde coverage
* Field names are stable across versions
* Schema version included in top-level envelopes
* Inbound strict via `deny_unknown_fields`

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsEnvelope {
    pub schema_version: u32,
    pub results: Results,
    pub meta: ResultsMeta,
}
```

### 4.2 Determinism Guarantees

Following overall.md §2.7 and §11.3:

```rust
// Python binding preserves determinism metadata
#[pymethods]
impl PyResults {
    #[getter]
    fn numeric_mode(&self) -> String {
        match self.inner.meta.numeric_mode {
            NumericMode::Decimal => "decimal",
            NumericMode::FastF64 => "fast_f64",
        }
    }
    
    #[getter]
    fn was_parallel(&self) -> bool {
        self.inner.meta.parallel
    }
}
```

### 4.3 Currency Safety

Enforcing overall.md §2.1 invariants:

```rust
// Both Python and WASM enforce same-currency arithmetic
impl PyMoney {
    fn __add__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        if self.inner.ccy != other.inner.ccy {
            return Err(PyErr::new::<PyValueError, _>(
                format!("Currency mismatch: {} != {}", 
                    self.inner.ccy, other.inner.ccy)
            ));
        }
        Ok(PyMoney { 
            inner: self.inner + other.inner 
        })
    }
}
```

### 4.4 Validation Framework Integration

Exposing core's validation framework (overall.md §2.4):

```python
from rfin import Validator, ValidationResult

validator = ModelValidator()
result = validator.validate(model)

if not result.passed:
    for warning in result.warnings:
        print(f"Warning: {warning.message} at {warning.location}")
```

---

## 5) Testing Strategy

### 5.1 Python Tests

```python
import pytest
from rfin import Currency, Money, FinstackError

class TestMoney:
    def test_currency_safety(self):
        """Test currency mismatch detection"""
        usd = Money(100, Currency.USD)
        eur = Money(100, Currency.EUR)
        
        with pytest.raises(ValueError, match="Currency mismatch"):
            usd + eur
    
    def test_determinism(self):
        """Test deterministic results"""
        results1 = model.evaluate(parallel=True)
        results2 = model.evaluate(parallel=False)
        
        assert results1.values == results2.values
        assert results1.numeric_mode == "decimal"
```

### 5.2 WASM Tests

```javascript
import { Currency, Money } from 'rfin-wasm';

describe('Money', () => {
    test('currency safety', () => {
        const usd = new Money(100, Currency.USD);
        const eur = new Money(100, Currency.EUR);
        
        expect(() => usd.add(eur)).toThrow('Currency mismatch');
    });
    
    test('serialization round-trip', () => {
        const original = new Money(123.45, Currency.USD);
        const json = original.toJSON();
        const restored = Money.fromJSON(json);
        
        expect(restored.equals(original)).toBe(true);
    });
});
```

### 5.3 Cross-Platform Validation

Golden tests ensure consistency across bindings:

```rust
#[test]
fn golden_test_results() {
    let model = create_test_model();
    let results = model.evaluate();
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&results).unwrap();
    
    // Compare with golden file
    let golden = include_str!("golden/model_results.json");
    assert_eq!(json, golden);
}
```

---

## 6) Build and Distribution

### 6.1 Python Distribution

Using maturin for wheel building:

```toml
[build-system]
requires = ["maturin>=1.0"]
build-backend = "maturin"

[tool.maturin]
python-source = "python"
module-name = "rfin._rfin"
features = ["pyo3/extension-module"]
```

CI matrix (overall.md §15.2):
* OS: Linux, macOS, Windows
* Python: 3.10, 3.11, 3.12
* Architectures: x86_64, aarch64 (Apple Silicon)

### 6.2 WASM Distribution

Multiple targets for different environments:

```bash
# Web browsers
wasm-pack build --target web --out-dir pkg

# Node.js
wasm-pack build --target nodejs --out-dir pkg-node

# Bundlers (webpack, rollup)
wasm-pack build --target bundler --out-dir pkg-bundler
```

NPM package configuration:

```json
{
  "name": "rfin-wasm",
  "version": "0.1.0",
  "files": ["pkg"],
  "main": "pkg/rfin_wasm.js",
  "types": "pkg/rfin_wasm.d.ts",
  "sideEffects": false
}
```

---

## 7) Documentation

### 7.0 Quickstarts (reuse over reimplementation)

Python quickstart (statements → valuations → portfolio):

```python
from rfin import Currency
from rfin.statements import FinancialModel
from rfin.valuations import MarketData, Bond
from rfin.portfolio import Portfolio, Position, PortfolioRunner

plan = "2025Q1..2026Q4"
model = (FinancialModel.builder("Acme")
    .periods(plan, actuals="2025Q1..Q2")
    .compute("gross_margin", "gross_profit / revenue")
    .register_metrics("fin.basic")
    .build())

market = MarketData(as_of="2025-01-01", curves={...}, fx={...})
bond = Bond.builder("AAPL-5Y").coupon(0.04).maturity("2030-01-25").build()

portfolio = (Portfolio.builder("Fund A")
    .plan(Currency.USD, "2025-01-01", FinancialModel.periods_of(model))
    .entity("OpCo", {"model": model})
    .position(Position(id="B1", entity="OpCo", instrument="AAPL-5Y", quantity=1_000_000, unit="face_value", open_date="2025-01-01"))
    .build())

results = PortfolioRunner(parallel=False).run(portfolio, market)
print(results.valuation.portfolio_total_base)
```

Browser quickstart (WASM):

```javascript
import { Portfolio, PortfolioRunner, Currency } from 'rfin-wasm';

const portfolio = /* build via builder helpers; calls into Rust */
const market = /* construct MarketData JSON; serde_wasm_bindgen maps it */
const runner = new PortfolioRunner({ parallel: false });
const out = await runner.run(portfolio, market);
console.log(out.valuation.portfolio_total_base);
```

### 7.1 Python Documentation

Sphinx-compatible docstrings:

```python
class FinancialModel:
    """
    Financial model for entity statements.
    
    A FinancialModel represents the financial statements and metrics
    for a single entity over multiple periods.
    
    Parameters
    ----------
    id : str
        Unique identifier for the model
    periods : List[Period]
        Evaluation periods
    nodes : Dict[str, Node]
        Statement nodes
        
    Examples
    --------
    >>> model = FinancialModel.builder("Acme") \\
    ...     .periods("2025Q1..2026Q4") \\
    ...     .compute("margin", "profit / revenue") \\
    ...     .build()
    """
```

### 7.2 JavaScript Documentation

JSDoc/TypeScript declarations:

```typescript
/**
 * Financial model for entity statements.
 * @class
 */
export class FinancialModel {
    /**
     * Create a new financial model
     * @param {string} id - Unique identifier
     * @param {Period[]} periods - Evaluation periods
     * @param {Map<string, Node>} nodes - Statement nodes
     */
    constructor(id: string, periods: Period[], nodes: Map<string, Node>);
    
    /**
     * Evaluate the model
     * @param {EvaluationOptions} options - Evaluation options
     * @returns {Results} Evaluation results
     */
    evaluate(options?: EvaluationOptions): Results;
}
```

---

## 8) Performance Considerations

### 8.1 Benchmarks

Target performance from overall.md §11.1:

```python
import time
from rfin import FinancialModel

# Build large model (10k nodes × 60 periods)
model = build_large_model()

start = time.time()
results = model.evaluate(parallel=False)
elapsed = time.time() - start

assert elapsed < 0.250  # < 250ms single-threaded
```

### 8.2 Memory Optimization

Python memory management with Arc for shared data:

```rust
use std::sync::Arc;

#[pyclass]
struct PyLargeDataset {
    // Share immutable data across Python objects
    inner: Arc<LargeDataset>,
}
```

WASM memory optimization:

```rust
// Use wee_alloc for smaller WASM size
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

---

## 9) Migration and Compatibility

### 9.1 Version Management

Following semver as per overall.md §12:

```rust
// Python version exposure
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__api_version__", "1.0")?;
    // ...
}
```

### 9.2 Deprecation Strategy

```python
import warnings

def deprecated_method(self):
    """
    .. deprecated:: 0.4.0
       Use :meth:`new_method` instead.
    """
    warnings.warn(
        "deprecated_method is deprecated, use new_method instead",
        DeprecationWarning,
        stacklevel=2
    )
    return self.new_method()
```

---

## 10) Future Enhancements

### 10.1 Async Support

Python async integration:

```python
async def evaluate_async(model: FinancialModel) -> Results:
    """Evaluate model asynchronously"""
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(None, model.evaluate)
```

### 10.2 Streaming APIs

For large datasets:

```python
from rfin import StreamingPortfolio

# Stream results as they're computed
async for result in portfolio.evaluate_streaming():
    print(f"Processed {result.entity_id}: {result.value}")
```

### 10.3 Plugin System

Expose analysis plugin system (overall.md §5):

```python
from rfin.analysis import Analyzer, register_analyzer

@register_analyzer("custom_analysis")
class CustomAnalyzer(Analyzer):
    def analyze(self, model, args):
        # Custom analysis implementation
        return results
```

---

## Acceptance Criteria

Aligned with `overall.md` §16 and crate‑specific acceptance criteria:

### Python Bindings
- [ ] All core types wrapped with Pythonic API
- [ ] Serde round-trip with pydantic models
- [ ] GIL released for heavy computations
- [ ] Feature-gated modules for optional functionality
- [ ] Comprehensive pytest suite with > 90% coverage
- [ ] Wheels for Python 3.10-3.12 on major platforms

### WASM Bindings
- [ ] JavaScript-friendly API with constructors
- [ ] Serde-wasm-bindgen for complex types
- [ ] Multiple build targets (web, Node.js, bundler)
- [ ] TypeScript declarations generated
- [ ] Bundle size < 500KB for core features
- [ ] Cross-browser compatibility tests

### Shared Requirements
- [ ] 100% wire format stability
- [ ] Currency safety enforced
- [ ] Deterministic results preserved
- [ ] Error messages user-friendly
- [ ] Documentation with examples
- [ ] Performance targets met

---

**This document serves as the authoritative specification for the RustFin bindings crates**, defining the interface between the core Rust library and Python/JavaScript ecosystems while maintaining the invariants and design principles established in the overall architecture.
