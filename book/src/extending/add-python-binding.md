# Add a Python Binding

This guide covers adding a PyO3 wrapper for a Rust type, generating the
`.pyi` stub, and writing a parity test.

## Step 1: Create the Wrapper

Add a new file in `finstack-py/src/` matching the module structure:

```rust,no_run
use pyo3::prelude::*;
use crate::errors::map_error;

/// Python wrapper for MyDerivative.
#[pyclass]
pub struct MyDerivative {
    /// The wrapped Rust type.
    pub(crate) inner: finstack_valuations::MyDerivative,
}

impl MyDerivative {
    pub(crate) fn from_inner(
        inner: finstack_valuations::MyDerivative,
    ) -> Self {
        Self { inner }
    }
}
```

## Step 2: Add `#[pymethods]`

```rust,no_run
#[pymethods]
impl MyDerivative {
    /// Get the instrument ID.
    fn id(&self) -> &str {
        self.inner.id()
    }

    /// Compute NPV.
    fn value(&self, market: &MarketContext, as_of: &PyDate) -> PyResult<Money> {
        let date = py_date_to_date(as_of)?;
        self.inner
            .value(&market.inner, date)
            .map(|m| Money::from_inner(m))
            .map_err(map_error)
    }

    fn __repr__(&self) -> String {
        format!("MyDerivative('{}')", self.inner.id())
    }
}
```

## Step 3: Builder Wrapper

```rust,no_run
#[pyclass]
pub struct MyDerivativeBuilder {
    pub(crate) inner: finstack_valuations::MyDerivativeBuilder,
}

#[pymethods]
impl MyDerivativeBuilder {
    #[staticmethod]
    fn builder(id: &str) -> Self {
        Self { inner: finstack_valuations::MyDerivativeBuilder::new(id) }
    }

    fn notional(&mut self, m: &Money) -> PyResult<Self> {
        Ok(Self { inner: self.inner.clone().notional(m.inner.clone()) })
    }

    fn build(&self) -> PyResult<MyDerivative> {
        self.inner.clone().build()
            .map(MyDerivative::from_inner)
            .map_err(map_error)
    }
}
```

## Step 4: Register the Module

```rust,no_run
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "my_module")?;
    m.add_class::<MyDerivative>()?;
    m.add_class::<MyDerivativeBuilder>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
```

## Step 5: Write the `.pyi` Stub

Add to `finstack-py/finstack/` matching the module path:

```python
class MyDerivative:
    def id(self) -> str: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...

class MyDerivativeBuilder:
    @staticmethod
    def builder(id: str) -> MyDerivativeBuilder: ...
    def notional(self, m: Money) -> MyDerivativeBuilder: ...
    def build(self) -> MyDerivative: ...
```

## Step 6: Parity Test

Add a test in `finstack-py/tests/parity/`:

```python
def test_my_derivative_parity():
    """Verify Python binding matches Rust output."""
    inst = MyDerivativeBuilder.builder("TEST") \
        .notional(Money(1_000_000, "USD")) \
        .build()

    result = registry.price_with_metrics(
        inst, "my_model", market, as_of,
        metrics=["dv01"],
    )

    assert result.npv.amount != 0.0
    assert "dv01" in result.metrics
```

## Checklist

- [ ] `#[pyclass]` wrapper with `pub(crate) inner`
- [ ] `from_inner()` constructor
- [ ] All methods use `map_err(map_error)` (never `.unwrap()`)
- [ ] `__repr__` implemented
- [ ] Module registered via `register()` function
- [ ] `.pyi` stub with full type annotations
- [ ] Parity test in `tests/parity/`
- [ ] `__all__` and `__doc__` set in module
