use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Re-export symbols from a submodule to the parent module.
///
/// This helper flattens nested module structures, allowing users to import
/// symbols from either `finstack.core.dates.Calendar` or `finstack.core.dates.calendar.Calendar`.
#[allow(dead_code)]
pub(crate) fn reexport_from_submodule(
    parent: &Bound<'_, PyModule>,
    submodule_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    let submodule = parent.getattr(submodule_name)?;
    for &export in exports {
        if let Ok(obj) = submodule.getattr(export) {
            parent.setattr(export, obj)?;
        }
    }
    Ok(())
}

