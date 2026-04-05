# Python Bindings

The `finstack-py` crate provides Python bindings via PyO3. The binding layer
follows strict conventions to ensure consistency and maintainability.

## Wrapper Pattern

Every Python type wraps a Rust type with a `pub(crate) inner` field:

```rust,no_run
#[pyclass]
pub struct DiscountCurve {
    /// The wrapped Rust discount curve
    pub(crate) inner: finstack_core::market_data::DiscountCurve,
}

impl DiscountCurve {
    /// Construct from the inner Rust type
    pub(crate) fn from_inner(
        inner: finstack_core::market_data::DiscountCurve
    ) -> Self {
        Self { inner }
    }
}
```

## Error Mapping

All Rust errors are mapped to Python exceptions via centralized helpers:

```rust,no_run
use crate::errors::map_error;

#[pymethods]
impl DiscountCurve {
    fn df(&self, t: f64) -> PyResult<f64> {
        self.inner.df(t).map_err(map_error)
    }
}
```

The `map_error` function converts `finstack_core::Error` variants to
appropriate Python exception types (`ValueError`, `RuntimeError`, etc.).

## Module Registration

Every submodule follows the same registration pattern:

```rust,no_run
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "term_structures")?;
    m.add_class::<DiscountCurve>()?;
    m.add_class::<ForwardCurve>()?;
    m.add_class::<HazardCurve>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
```

## `.pyi` Stub Files

Type stubs are manually maintained in `finstack-py/finstack/`. They provide:
- Full type annotations for IDE autocompletion
- Docstrings for hover documentation
- PEP 561 compliance (`py.typed` marker)

Stub structure mirrors the Python module hierarchy:

```text
finstack-py/finstack/
├── __init__.pyi
├── core/
│   ├── __init__.pyi
│   ├── currency.pyi
│   ├── money.pyi
│   └── market_data/
│       ├── term_structures.pyi
│       └── surfaces.pyi
├── valuations/
│   ├── instruments.pyi
│   └── pricer.pyi
└── ...
```

## Builder Pattern

Python builders mirror the Rust fluent API:

```python
# Python
bond = Bond.builder("BOND_001") \
    .money(Money(1_000_000, "USD")) \
    .coupon_rate(0.045) \
    .build()
```

```rust,no_run
// Rust binding
#[pyclass]
pub struct BondBuilder {
    pub(crate) inner: finstack_valuations::BondBuilder,
}

#[pymethods]
impl BondBuilder {
    fn money(&mut self, money: &Money) -> PyResult<Self> {
        // clone + set + return self for chaining
    }

    fn build(&self) -> PyResult<Bond> {
        self.inner.build().map(Bond::from_inner).map_err(map_error)
    }
}
```

## Parity Tests

Parity tests under `finstack-py/tests/parity/` verify that Python bindings
produce identical results to direct Rust calls for every instrument type
and metric.
