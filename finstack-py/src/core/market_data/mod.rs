pub(crate) mod context;
pub(crate) mod dividends;
pub(crate) mod fx;
pub(crate) mod interp;
pub(crate) mod scalars;
pub(crate) mod surfaces;
pub(crate) mod term_structures;

#[allow(unused_imports)]
pub use context::PyMarketContext;
#[allow(unused_imports)]
pub use dividends::{PyDividendEvent, PyDividendSchedule, PyDividendScheduleBuilder};
#[allow(unused_imports)]
pub use fx::{PyFxConfig, PyFxConversionPolicy, PyFxMatrix, PyFxRateResult};
#[allow(unused_imports)]
pub use interp::{PyExtrapolationPolicy, PyInterpStyle};
#[allow(unused_imports)]
pub use scalars::{PyMarketScalar, PyScalarTimeSeries, PySeriesInterpolation};
#[allow(unused_imports)]
pub use surfaces::PyVolSurface;
#[allow(unused_imports)]
pub use term_structures::{
    PyBaseCorrelationCurve, PyCreditIndexData, PyDiscountCurve, PyForwardCurve, PyHazardCurve,
    PyInflationCurve,
};

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::collections::HashSet;

fn reexport_from_submodule(
    parent: &Bound<'_, PyModule>,
    submodule: &str,
    names: &[&'static str],
) -> PyResult<()> {
    let handle = parent.getattr(submodule)?;
    let module = handle.downcast::<PyModule>()?;
    for &name in names {
        let value = module.getattr(name)?;
        parent.setattr(name, value)?;
    }
    Ok(())
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "market_data")?;
    module.setattr(
        "__doc__",
        "Market data components mirroring finstack-core: curves, surfaces, FX, scalars, and aggregation context.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let interp_exports = interp::register(py, &module)?;
    reexport_from_submodule(&module, "interp", &interp_exports)?;
    exports.extend(interp_exports.iter().copied());

    let term_exports = term_structures::register(py, &module)?;
    reexport_from_submodule(&module, "term_structures", &term_exports)?;
    exports.extend(term_exports.iter().copied());

    let surface_exports = surfaces::register(py, &module)?;
    reexport_from_submodule(&module, "surfaces", &surface_exports)?;
    exports.extend(surface_exports.iter().copied());

    let scalar_exports = scalars::register(py, &module)?;
    reexport_from_submodule(&module, "scalars", &scalar_exports)?;
    exports.extend(scalar_exports.iter().copied());

    let dividend_exports = dividends::register(py, &module)?;
    reexport_from_submodule(&module, "dividends", &dividend_exports)?;
    exports.extend(dividend_exports.iter().copied());

    let fx_exports = fx::register(py, &module)?;
    reexport_from_submodule(&module, "fx", &fx_exports)?;
    exports.extend(fx_exports.iter().copied());

    let context_exports = context::register(py, &module)?;
    reexport_from_submodule(&module, "context", &context_exports)?;
    exports.extend(context_exports.iter().copied());

    let mut uniq = HashSet::new();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

    Ok(())
}
