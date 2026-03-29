# Python Binding Code Reviewer

## Quick Start

When reviewing Python binding code in `finstack-py/`, produce a review that ensures:

1. **Summary**: What changed, risk of logic leaking into Python.
2. **Binding concerns**: 3–7 bullets on architecture violations, type conversion issues, and pattern consistency.
3. **Findings**: Grouped by severity with concrete fixes.
4. **Action items**: Checklist with specific improvements.

After each review cycle, re-check and update. Continue until no action items remain.

## Core Principle

**All logic stays in Rust. Python does only:**
- Type conversion (Python → Rust, Rust → Python)
- Wrapper construction and accessor methods
- Error mapping to Python exceptions
- Ergonomic helpers (operator overloading, flexible argument parsing)

**Why?** The WASM bindings need identical Rust functionality. Any logic in Python must be reimplemented for WASM, causing drift and bugs.

## Severity Rubric

| Severity | Definition |
|----------|------------|
| **Blocker** | Business logic in Python, computation outside Rust, algorithm implementation in bindings |
| **Major** | Validation logic that should be in Rust, inconsistent patterns, missing error mapping |
| **Minor** | Suboptimal type conversion, missing docstrings, inconsistent naming |
| **Nit** | Style preference, minor documentation improvements |

## What Belongs in Python Bindings

### Acceptable (GOOD)

```rust
// Type wrapper with inner field
#[pyclass(name = "Money", module = "finstack.core.money", frozen)]
pub struct PyMoney {
    pub(crate) inner: Money,
}

// Accessor methods - just expose Rust data
#[pymethods]
impl PyMoney {
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount()
    }
}

// Flexible type extraction - accepts multiple Python types
impl<'py> FromPyObject<'py> for CurrencyArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(ccy) = obj.extract::<PyRef<PyCurrency>>() {
            return Ok(CurrencyArg(ccy.inner));
        }
        if let Ok(code) = obj.extract::<&str>() {
            return Currency::from_str(code)
                .map(CurrencyArg)
                .map_err(|_| errors::unknown_currency(code));
        }
        Err(PyTypeError::new_err("Expected Currency or string"))
    }
}

// Error mapping to Python exceptions
pub fn map_error(e: CoreError) -> PyErr {
    match e {
        CoreError::Config(msg) => ConfigurationError::new_err(msg),
        CoreError::Compute(msg) => ComputationError::new_err(msg),
        // ...
    }
}
```

### NOT Acceptable (BAD)

```rust
// BAD: Business logic in binding
#[pymethods]
impl PyBond {
    fn calculate_yield(&self, price: f64) -> f64 {
        // Newton-Raphson iteration - THIS SHOULD BE IN RUST CORE
        let mut y = 0.05;
        for _ in 0..100 {
            let f = self.price_at_yield(y) - price;
            let df = self.price_sensitivity(y);
            y = y - f / df;
        }
        y
    }
}

// BAD: Validation logic that should be in Rust
#[pymethods]
impl PySwap {
    fn new(notional: f64, rate: f64) -> PyResult<Self> {
        // Validation should be in Rust, not Python binding
        if notional <= 0.0 {
            return Err(PyValueError::new_err("Notional must be positive"));
        }
        if rate < -0.10 || rate > 0.50 {
            return Err(PyValueError::new_err("Rate out of range"));
        }
        // ...
    }
}

// BAD: Data transformation logic
#[pymethods]
impl PyPortfolio {
    fn aggregate_by_currency(&self) -> HashMap<String, f64> {
        // Aggregation logic should be in Rust
        let mut result = HashMap::new();
        for pos in &self.positions {
            *result.entry(pos.currency()).or_insert(0.0) += pos.value();
        }
        result
    }
}
```

## Review Checklist

### Architecture Compliance

- [ ] No algorithms or computations implemented in bindings
- [ ] No business logic or financial calculations
- [ ] No validation logic (beyond type conversion)
- [ ] All computation delegated to Rust core crates
- [ ] Pattern consistency with WASM bindings

### Wrapper Pattern

- [ ] Uses `pub(crate) inner: RustType` pattern
- [ ] Provides `from_inner()` constructor for internal use
- [ ] Accessor methods just return `self.inner.field()`
- [ ] Methods delegate to `self.inner.method().map_err(map_error)`

### Type Conversion

- [ ] Flexible argument parsing (accepts objects, strings, tuples)
- [ ] Clear error messages on type mismatch
- [ ] Supports pandas/numpy interop where appropriate
- [ ] Uses centralized `args.rs` patterns

### Error Handling

- [ ] Uses centralized `map_error()` function
- [ ] Maps to appropriate exception hierarchy
- [ ] Preserves error context from Rust

### Module Organization

- [ ] Mirrors Rust crate structure
- [ ] Uses consistent `register()` function pattern
- [ ] Sets `__all__` and `__doc__` attributes
- [ ] Comprehensive docstrings with examples

### Python Files (`finstack/`)

- [ ] Only contains `.pyi` stubs (type hints)
- [ ] Ergonomic helpers only (no computation)
- [ ] DSL parsers that produce Rust objects (acceptable)

## Red Flags

| Issue | Symptom | Fix |
|-------|---------|-----|
| Logic in binding | Loop/iteration computing values | Move to Rust core, call from binding |
| Validation in binding | `if value < X` checks with custom errors | Add validation to Rust constructor |
| Data transformation | Map/filter/reduce on collections | Create Rust method, wrap result |
| Algorithm implementation | Math operations, financial formulas | Must be in Rust core crate |
| Inconsistent patterns | Different wrapper styles in same module | Align with established patterns |
| Missing error mapping | Direct `.unwrap()` or panic | Use `map_err(map_error)` |

## WASM Parity Check

When reviewing Python bindings, verify the functionality could be replicated in WASM:

```rust
// Python binding
#[pymethods]
impl PyBond {
    fn dirty_price(&self, market_data: &PyMarketData) -> PyResult<f64> {
        self.inner.dirty_price(&market_data.inner).map_err(map_error)
    }
}

// WASM equivalent should be straightforward:
#[wasm_bindgen]
impl JsBond {
    pub fn dirty_price(&self, market_data: &JsMarketData) -> Result<f64, JsValue> {
        self.inner.dirty_price(&market_data.inner).map_err(core_to_js)
    }
}
```

If the Python implementation can't be trivially replicated in WASM, it's a sign that logic has leaked into the binding layer.

## Review Output Template

```markdown
## Summary
<1–3 bullets: what changed, risk of logic leakage>

## Binding Concerns
- <architecture violation or pattern issue>
- <type conversion concern>

## Findings

### Blockers
- <logic in Python> (move to Rust: specific location suggestion)

### Majors
- <pattern inconsistency> (align with: specific example file)

### Minors / Nits
- <documentation improvement>

## Action Items
- [ ] <specific code to move to Rust>
- [ ] <pattern to align>
- [ ] <test to add>
```

## Additional Resources

- For detailed patterns and examples, see [reference.md](reference.md)
- For common issues and fixes, see [examples.md](examples.md)
