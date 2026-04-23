---
trigger: manual
description:
globs:
---

# Core Bindings Guide — Structure and How to Add New Features

This guide explains the structure of the `finstack-py` core bindings and how to add or extend features.

## Directory Structure

All core bindings live under `finstack-py/src/bindings/core/`:

```
finstack-py/src/bindings/core/
  mod.rs          # register() — registers all core submodules
  currency.rs     # ISO‑4217 currencies and helpers
  money.rs        # Money wrapper, formatting, arithmetic
  dates/          # business calendars, day‑counts, schedules, IMM, periods
  market_data/    # term structures, surfaces, scalars, FX, context
  math/           # integration, solvers, distributions
```

The root entrypoint `finstack-py/src/lib.rs` delegates to `bindings::register_root` which calls `core::register`.

## Registration Pattern

Each leaf module defines a `register(py, parent)` function that:
- Creates a submodule with `PyModule::new`
- Adds classes/functions with docstrings
- Sets `__all__` via `PyList`

Parent modules call submodule `register` functions.

## Adding a New Core Feature

1. Identify the existing Rust core API (in `finstack/core/...`) you need to expose.
2. Create a new bindings file under `src/bindings/core/<matching_module_path>.rs`.
3. Define Python‑exposed types using PyO3:
   - Use `#[pyclass(module = "finstack.core.<subpath>", name = "PublicName")]` for classes
   - Use `#[pymethods]` and `#[pyfunction]` for methods and free functions
   - Provide `#[pyo3(text_signature = "...")]` for public callables
   - **Type and function names must match Rust exactly** (e.g. Rust `Date` → Python `Date`, not `FsDate`)
4. Map errors using `core_to_py` from `errors.rs`.
5. Add a `register` function to create and attach the submodule.
6. Update the parent `mod.rs` to include the new submodule and call its `register`.

## Example Template

```rust
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyclass(module = "finstack.core.market_data", name = "NewType", frozen)]
#[derive(Clone)]
pub struct PyNewType {
    pub(crate) inner: finstack_core::market_data::NewType,
}

#[pymethods]
impl PyNewType {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn new(id: &str) -> PyResult<Self> {
        let inner = finstack_core::market_data::NewType::new(id).map_err(crate::errors::core_to_py)?;
        Ok(Self { inner })
    }

    #[getter]
    fn id(&self) -> String { self.inner.id().to_string() }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "new_feature")?;
    module.setattr("__doc__", "New feature from finstack_core::market_data")?;
    module.add_class::<PyNewType>()?;
    module.setattr("__all__", PyList::new(py, ["NewType"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("new_feature", &module)?;
    Ok(())
}
```

## Conventions

- Do mirror Rust types and names exactly; avoid inventing Python‑only types.
- Don't duplicate business logic; call into Rust crates and surface results.
- Do accept ergonomic inputs (e.g., `Currency` or `str` code).
- Don't silently coerce incompatible types; raise `TypeError` or `ValueError`.
- Do keep containers immutable; provide builders for fluent mutation.

## Review Checklist

- [ ] Mirrors an existing core API (no new business logic).
- [ ] `text_signature` present; docstrings complete.
- [ ] Errors mapped via `core_to_py`.
- [ ] Registered under `finstack-py/src/bindings/core/`.
- [ ] Type/function names match Rust exactly.
