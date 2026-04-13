//! Python bindings for `finstack_core::market_data` term structures and context.

pub mod context;
pub mod curves;
pub mod fx;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Promote all names from a submodule's `__all__` onto the parent module.
fn promote_from_submodule(parent: &Bound<'_, PyModule>, submod_name: &str) -> PyResult<()> {
    let sub = parent.getattr(submod_name)?;
    let all = match sub.getattr("__all__") {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let names: Vec<String> = all.extract()?;
    for name in names {
        if let Ok(obj) = sub.getattr(name.as_str()) {
            parent.add(name.as_str(), obj)?;
        }
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

    promote_from_submodule(&m, "curves")?;
    promote_from_submodule(&m, "fx")?;
    promote_from_submodule(&m, "context")?;

    let all = PyList::new(
        py,
        [
            "curves",
            "fx",
            "context",
            "DiscountCurve",
            "ForwardCurve",
            "HazardCurve",
            "InflationCurve",
            "PriceCurve",
            "VolSurface",
            "VolatilityIndexCurve",
            "FxMatrix",
            "FxRateResult",
            "FxConversionPolicy",
            "MarketContext",
        ],
    )?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;

    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
