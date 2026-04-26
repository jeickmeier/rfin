//! Python bindings for `finstack_core::market_data` term structures and context.

pub mod arbitrage;
pub mod context;
pub mod curves;
pub mod dtsm;
pub mod fx;

use pyo3::prelude::*;
use pyo3::types::PyList;

const ROOT_SUBMODULES: &[&str] = &["curves", "fx", "context", "dtsm", "arbitrage"];

/// Promote an explicit export list from a submodule onto the parent module.
fn promote_exports(
    parent: &Bound<'_, PyModule>,
    submod_name: &str,
    exports: &[&str],
) -> PyResult<()> {
    let sub = parent.getattr(submod_name)?;
    for name in exports {
        let obj = sub.getattr(*name)?;
        parent.add(*name, obj)?;
    }
    Ok(())
}

/// Register the `finstack.core.market_data` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "market_data")?;
    m.setattr(
        "__doc__",
        "Bindings for finstack-core market data: curves, vol surfaces, FX, and market context.",
    )?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core".to_string(),
        },
        Err(_) => "finstack.core".to_string(),
    };
    let qual = format!("{pkg}.market_data");
    m.setattr("__package__", &qual)?;

    curves::register(py, &m)?;
    fx::register(py, &m)?;
    context::register(py, &m)?;
    dtsm::register(py, &m)?;
    arbitrage::register(py, &m)?;

    promote_exports(&m, "curves", curves::EXPORTS)?;
    promote_exports(&m, "fx", fx::EXPORTS)?;
    promote_exports(&m, "context", context::EXPORTS)?;

    let mut all_names = ROOT_SUBMODULES.to_vec();
    all_names.extend_from_slice(curves::EXPORTS);
    all_names.extend_from_slice(fx::EXPORTS);
    all_names.extend_from_slice(context::EXPORTS);

    let all = PyList::new(py, &all_names)?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;

    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
