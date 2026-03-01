use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Re-export symbols from a named submodule onto the parent module.
///
/// Flattens nested module structures so users can import from either
/// `finstack.core.dates.Calendar` or `finstack.core.dates.calendar.Calendar`.
pub(crate) fn promote_exports<'py>(
    parent: &Bound<'py, PyModule>,
    submodule_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    if exports.is_empty() {
        return Ok(());
    }

    let submodule = parent
        .getattr(submodule_name)?
        .downcast_into::<PyModule>()?;
    for &name in exports {
        if submodule.hasattr(name)? {
            let attr = submodule.getattr(name)?;
            parent.setattr(name, attr)?;
        }
    }
    Ok(())
}
