pub(crate) mod ids;
pub(crate) mod registry;
pub(crate) mod risk;

pub(crate) use ids::MetricIdArg;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

fn promote_exports<'py>(
    parent: &Bound<'py, PyModule>,
    submodule_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    let submodule_any = parent.getattr(submodule_name)?;
    let submodule = submodule_any.cast::<PyModule>()?;
    for &name in exports {
        if submodule.hasattr(name)? {
            let attr = submodule.getattr(name)?;
            parent.setattr(name, attr)?;
        }
    }
    Ok(())
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "metrics")?;
    module.setattr(
        "__doc__",
        "Metric identifiers and registry helpers for finstack valuations.",
    )?;

    let ids_exports = ids::register(py, &module)?;
    promote_exports(&module, "ids", &ids_exports)?;

    let registry_exports = registry::register(py, &module)?;
    promote_exports(&module, "registry", &registry_exports)?;

    let mut exports: Vec<&str> = Vec::new();
    exports.extend(ids_exports.iter().copied());
    exports.extend(registry_exports.iter().copied());
    exports.sort_unstable();
    exports.dedup();

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
