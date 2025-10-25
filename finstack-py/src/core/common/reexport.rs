use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Re-export symbols from a registered PyO3 submodule onto the parent module.
#[inline]
pub fn reexport_from_submodule(
    parent: &Bound<'_, PyModule>,
    submodule: &str,
    names: &[&'static str],
) -> PyResult<()> {
    let handle = parent.getattr(submodule)?;
    let module = handle.downcast::<PyModule>()?;
    for &name in names {
        let value = module.getattr(name)?;
        parent.setattr(name, value)?;
    }
    Ok(())
}
