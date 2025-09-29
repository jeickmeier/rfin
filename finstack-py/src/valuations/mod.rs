pub(crate) mod calibration;
pub(crate) mod cashflow;
pub(crate) mod common;
pub(crate) mod instruments;
pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod results;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::collections::HashSet;

fn reexport_from_submodule(
    parent: &Bound<'_, PyModule>,
    submodule: &str,
    names: &[&'static str],
) -> PyResult<()> {
    if names.is_empty() {
        return Ok(());
    }
    let handle = parent.getattr(submodule)?;
    let module = handle.downcast::<PyModule>()?;
    for &name in names {
        let value = module.getattr(name)?;
        parent.setattr(name, value)?;
    }
    Ok(())
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "valuations")?;
    module.setattr(
        "__doc__",
        "Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let common_exports = common::register(py, &module)?;
    reexport_from_submodule(&module, "common", &common_exports)?;
    exports.extend(common_exports.iter().copied());

    let cashflow_exports = cashflow::register(py, &module)?;
    reexport_from_submodule(&module, "cashflow", &cashflow_exports)?;
    exports.extend(cashflow_exports.iter().copied());

    let results_exports = results::register(py, &module)?;
    reexport_from_submodule(&module, "results", &results_exports)?;
    exports.extend(results_exports.iter().copied());

    let pricer_exports = pricer::register(py, &module)?;
    reexport_from_submodule(&module, "pricer", &pricer_exports)?;
    exports.extend(pricer_exports.iter().copied());

    let metrics_exports = metrics::register(py, &module)?;
    reexport_from_submodule(&module, "metrics", &metrics_exports)?;
    exports.extend(metrics_exports.iter().copied());

    let instrument_exports = instruments::register(py, &module)?;
    reexport_from_submodule(&module, "instruments", &instrument_exports)?;
    exports.extend(instrument_exports.iter().copied());

    let calibration_exports = calibration::register(py, &module)?;
    reexport_from_submodule(&module, "calibration", &calibration_exports)?;
    exports.extend(calibration_exports.iter().copied());

    let mut uniq = HashSet::new();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("valuations", &module)?;
    Ok(())
}
