pub(crate) mod attribution;
pub(crate) mod calibration;
pub(crate) mod cashflow;
pub(crate) mod common;
pub(crate) mod covenants;
pub(crate) mod dataframe;
pub(crate) mod instruments;
pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod results;
pub(crate) mod risk;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::collections::HashSet;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "valuations")?;
    module.setattr(
        "__doc__",
        "Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let common_exports = common::register(py, &module)?;
    exports.extend(common_exports.iter().copied());
    // Note: common::register already calls parent.setattr("common", &module)

    let cashflow_exports = cashflow::register(py, &module)?;
    exports.extend(cashflow_exports.iter().copied());

    let results_exports = results::register(py, &module)?;
    exports.extend(results_exports.iter().copied());

    let pricer_exports = pricer::register(py, &module)?;
    exports.extend(pricer_exports.iter().copied());

    let metrics_exports = metrics::register(py, &module)?;
    exports.extend(metrics_exports.iter().copied());

    let instrument_exports = instruments::register(py, &module)?;
    exports.extend(instrument_exports.iter().copied());

    let calibration_exports = calibration::register(py, &module)?;
    exports.extend(calibration_exports.iter().copied());

    let dataframe_exports = dataframe::register(py, &module)?;
    exports.extend(dataframe_exports.iter().copied());

    let risk_exports = risk::register(py, &module)?;
    exports.extend(risk_exports.iter().copied());

    let cov_exports = covenants::register(py, &module)?;
    exports.extend(cov_exports.iter().copied());

    // Register attribution module (as submodule and re-export to valuations)
    let attr_submod = PyModule::new(py, "attribution")?;
    let attr_exports = attribution::register(&attr_submod)?;
    module.add_submodule(&attr_submod)?;
    module.setattr("attribution", &attr_submod)?;
    exports.extend(attr_exports.iter().copied());

    let mut uniq = HashSet::new();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("valuations", &module)?;
    Ok(())
}
