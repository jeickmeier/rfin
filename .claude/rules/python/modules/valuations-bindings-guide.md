---
trigger: manual
description:
globs:
---

# Valuations Bindings Guide — Structure and How to Add New Features

This guide covers the `finstack-py` valuations bindings.

## Scope and Principles

- Bindings expose valuation results and builders; computing logic lives in the Rust `finstack-valuations` crate.
- Currency‑safety: all cashflow math is currency‑preserving; explicit FX handling via `FxProvider`/`FxMatrix`.
- Determinism: serial ≡ parallel; Decimal numerics preserved by core.

## Directory Structure

All valuations bindings live under `finstack-py/src/bindings/valuations/`:

```
finstack-py/src/bindings/valuations/
  mod.rs            # register() — registers all valuations submodules
  instruments/      # instrument type wrappers (bonds, rates, equity, credit, etc.)
  calibration/      # calibration bindings
  margin/           # valuations.margin bindings (distinct from top-level finstack.margin)
  xva/              # XVA bindings
```

Module paths mirror the Rust crate: `finstack.valuations.instruments`, `finstack.valuations.calibration`, etc.

## Adding a New Instrument/Feature

1. Identify the core entry points in `finstack-valuations/src/...` to bind.
2. Create bindings under `src/bindings/valuations/<area>.rs` or a folder with `mod.rs`.
3. Expose Python classes/functions with PyO3:
   - `#[pyclass(module = "finstack.valuations.<area>", name = "PublicName")]`
   - **Type and function names must match Rust exactly.**
   - Provide constructor(s) that map 1:1 to Rust builders or pricers.
   - Methods that call core evaluation (e.g., `price`, `pv`, `cashflows`), returning typed results.
4. Map errors with `core_to_py`.
5. Add a `register(py, parent)` function; set `__all__` via `PyList`.
6. Update the parent `mod.rs` to include and call the new submodule.

## Result Types

- Prefer returning structured Python classes with clear getters (e.g., `pv`, `npv`, `dv01`).
- Include metadata: numeric mode, rounding context, FX policy when available from core.

## FX and Multi‑Currency Results

- Never convert currencies implicitly.
- Accept an `FxMatrix` and explicit `FxConversionPolicy` for base‑currency rollups.

## Example Template

```rust
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyclass(module = "finstack.valuations.instruments", name = "Bond", frozen)]
pub struct PyBond {
    pub(crate) inner: finstack_valuations::instruments::Bond,
}

#[pymethods]
impl PyBond {
    #[staticmethod]
    #[pyo3(text_signature = "(coupon, maturity, ...)")]
    fn fixed(/* params */) -> PyResult<Self> {
        let inner = finstack_valuations::instruments::Bond::fixed(/* ... */)
            .map_err(crate::errors::core_to_py)?;
        Ok(Self { inner })
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "instruments")?;
    module.add_class::<PyBond>()?;
    module.setattr("__all__", PyList::new(py, ["Bond"])?)?;
    parent.add_submodule(&module)?;
    parent.setattr("instruments", &module)?;
    Ok(())
}
```

## Review Checklist

- [ ] Mirrors a core valuation entry point (no Python‑side business logic).
- [ ] Errors mapped via `core_to_py`; no `unwrap` on user inputs.
- [ ] Type and function names match Rust exactly.
- [ ] Registered under `finstack-py/src/bindings/valuations/`.
- [ ] Result types are structured, with metadata where applicable.
