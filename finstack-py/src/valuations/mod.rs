pub(crate) mod attribution;
pub(crate) mod calibration;
pub(crate) mod cashflow;
pub(crate) mod common;
pub(crate) mod covenants;
pub(crate) mod dataframe;
pub(crate) mod instruments;
pub(crate) mod metrics;
pub(crate) mod performance;
pub(crate) mod pricer;
pub(crate) mod results;
pub(crate) mod risk;

use finstack_core::HashSet;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "valuations")?;
    module.setattr(
        "__doc__",
        "Valuations bindings mirroring finstack-valuations: instruments, pricers, metrics, and result envelopes.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let common_exports = common::register(py, &module)?;
    exports.extend(common_exports.iter().copied());
    promote_exports(&module, "common", &common_exports)?;

    let cashflow_exports = cashflow::register(py, &module)?;
    exports.extend(cashflow_exports.iter().copied());
    promote_exports(&module, "cashflow", &cashflow_exports)?;

    let results_exports = results::register(py, &module)?;
    exports.extend(results_exports.iter().copied());
    promote_exports(&module, "results", &results_exports)?;

    let pricer_exports = pricer::register(py, &module)?;
    exports.extend(pricer_exports.iter().copied());
    promote_exports(&module, "pricer", &pricer_exports)?;

    let metrics_exports = metrics::register(py, &module)?;
    exports.extend(metrics_exports.iter().copied());
    promote_exports(&module, "metrics", &metrics_exports)?;

    let performance_exports = performance::register(py, &module)?;
    exports.extend(performance_exports.iter().copied());
    promote_exports(&module, "performance", &performance_exports)?;

    let instrument_exports = instruments::register(py, &module)?;
    exports.extend(instrument_exports.iter().copied());
    promote_exports(&module, "instruments", &instrument_exports)?;

    let calibration_exports = calibration::register(py, &module)?;
    exports.extend(calibration_exports.iter().copied());
    promote_exports(&module, "calibration", &calibration_exports)?;

    let dataframe_exports = dataframe::register(py, &module)?;
    exports.extend(dataframe_exports.iter().copied());
    promote_exports(&module, "dataframe", &dataframe_exports)?;

    let risk_exports = risk::register(py, &module)?;
    exports.extend(risk_exports.iter().copied());
    promote_exports(&module, "risk", &risk_exports)?;

    let cov_exports = covenants::register(py, &module)?;
    exports.extend(cov_exports.iter().copied());
    promote_exports(&module, "covenants", &cov_exports)?;

    // Register attribution module (as submodule and re-export to valuations)
    let attr_submod = PyModule::new(py, "attribution")?;
    let attr_exports = attribution::register(&attr_submod)?;
    module.add_submodule(&attr_submod)?;
    module.setattr("attribution", &attr_submod)?;
    promote_exports(&module, "attribution", &attr_exports)?;
    exports.extend(attr_exports.iter().copied());

    let mut uniq = HashSet::default();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("valuations", &module)?;
    Ok(())
}

fn promote_exports<'py>(
    parent: &Bound<'py, PyModule>,
    submodule_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    if exports.is_empty() {
        return Ok(());
    }

    let submodule_any = parent.getattr(submodule_name)?;
    let submodule = submodule_any.downcast::<PyModule>()?;
    for &name in exports {
        if submodule.hasattr(name)? {
            let attr = submodule.getattr(name)?;
            parent.setattr(name, attr)?;
        }
    }
    Ok(())
}
