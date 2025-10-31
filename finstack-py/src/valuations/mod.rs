pub(crate) mod calibration;
pub(crate) mod cashflow;
pub(crate) mod common;
pub(crate) mod dataframe;
pub(crate) mod instruments;
pub(crate) mod mc_generator;
pub(crate) mod mc_params;
pub(crate) mod mc_paths;
pub(crate) mod mc_result;
pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod results;
pub(crate) mod risk;

use crate::core::common::reexport::reexport_from_submodule;
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

    let dataframe_exports = dataframe::register(py, &module)?;
    reexport_from_submodule(&module, "dataframe", &dataframe_exports)?;
    exports.extend(dataframe_exports.iter().copied());

    let risk_exports = risk::register(py, &module)?;
    reexport_from_submodule(&module, "risk", &risk_exports)?;
    exports.extend(risk_exports.iter().copied());

    let mc_paths_exports = mc_paths::register(py, &module)?;
    reexport_from_submodule(&module, "mc_paths", &mc_paths_exports)?;
    exports.extend(mc_paths_exports.iter().copied());

    let mc_params_exports = mc_params::register(py, &module)?;
    reexport_from_submodule(&module, "mc_params", &mc_params_exports)?;
    exports.extend(mc_params_exports.iter().copied());

    let mc_result_exports = mc_result::register(py, &module)?;
    reexport_from_submodule(&module, "mc_result", &mc_result_exports)?;
    exports.extend(mc_result_exports.iter().copied());

    let mc_generator_exports = mc_generator::register(py, &module)?;
    reexport_from_submodule(&module, "mc_generator", &mc_generator_exports)?;
    exports.extend(mc_generator_exports.iter().copied());

    let mut uniq = HashSet::new();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("valuations", &module)?;
    Ok(())
}
