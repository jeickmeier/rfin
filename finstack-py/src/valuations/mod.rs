pub(crate) mod attribution;
pub(crate) mod calibration;
pub(crate) mod cashflow;
pub(crate) mod common;
pub(crate) mod constants;
pub(crate) mod covenants;
pub(crate) mod instruments;
pub(crate) mod lsmc;
pub(crate) mod margin;
pub(crate) mod market;
pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod results;
pub(crate) mod schema;
pub(crate) mod xva;

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

    let performance_exports = cashflow::performance::register(py, &module)?;
    exports.extend(performance_exports.iter().copied());
    promote_exports(&module, "performance", &performance_exports)?;

    let instrument_exports = instruments::register(py, &module)?;
    exports.extend(instrument_exports.iter().copied());
    promote_exports(&module, "instruments", &instrument_exports)?;

    let calibration_exports = calibration::register(py, &module)?;
    exports.extend(calibration_exports.iter().copied());
    promote_exports(&module, "calibration", &calibration_exports)?;

    let dataframe_exports = results::dataframe::register(py, &module)?;
    exports.extend(dataframe_exports.iter().copied());
    promote_exports(&module, "dataframe", &dataframe_exports)?;

    let risk_exports = metrics::risk::register(py, &module)?;
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

    // Register conventions module
    let conventions_exports = market::conventions::register(py, &module)?;
    exports.extend(conventions_exports.iter().copied());
    promote_exports(&module, "conventions", &conventions_exports)?;

    // Register margin module
    let margin_exports = margin::register(py, &module)?;
    exports.extend(margin_exports.iter().copied());
    promote_exports(&module, "margin", &margin_exports)?;

    // Register market module (quote->instrument builders)
    let market_exports = market::register(py, &module)?;
    exports.extend(market_exports.iter().copied());
    promote_exports(&module, "market", &market_exports)?;

    // Register bumps module
    let bumps_exports = calibration::bumps::register(py, &module)?;
    exports.extend(bumps_exports.iter().copied());
    promote_exports(&module, "bumps", &bumps_exports)?;

    // Register LSMC (Longstaff-Schwartz Monte Carlo) pricer module
    let lsmc_exports = lsmc::register(py, &module)?;
    exports.extend(lsmc_exports.iter().copied());
    promote_exports(&module, "lsmc", &lsmc_exports)?;

    // Register schema module (JSON Schema helpers)
    let schema_exports = schema::register(py, &module)?;
    exports.extend(schema_exports.iter().copied());
    promote_exports(&module, "schema", &schema_exports)?;

    // Register constants module (numerical constants for pricing & risk)
    let constants_exports = constants::register(py, &module)?;
    exports.extend(constants_exports.iter().copied());
    promote_exports(&module, "constants", &constants_exports)?;

    // Register XVA module (CVA, exposure, netting, collateral)
    let xva_exports = xva::register(py, &module)?;
    exports.extend(xva_exports.iter().copied());
    promote_exports(&module, "xva", &xva_exports)?;

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
    let submodule = submodule_any.cast::<PyModule>()?;
    for &name in exports {
        if submodule.hasattr(name)? {
            let attr = submodule.getattr(name)?;
            parent.setattr(name, attr)?;
        }
    }
    Ok(())
}
