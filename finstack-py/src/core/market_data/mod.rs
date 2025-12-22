pub(crate) mod bumps;
pub(crate) mod context;
pub(crate) mod diff;
pub(crate) mod dividends;
pub(crate) mod fx;
pub(crate) mod scalars;
pub(crate) mod surfaces;
pub(crate) mod term_structures;
pub mod volatility;

#[allow(unused_imports)]
pub use context::PyMarketContext;
#[allow(unused_imports)]
pub use dividends::{PyDividendEvent, PyDividendSchedule, PyDividendScheduleBuilder};
#[allow(unused_imports)]
pub use fx::{PyFxConfig, PyFxConversionPolicy, PyFxMatrix, PyFxRateResult};
#[allow(unused_imports)]
pub use scalars::{PyMarketScalar, PyScalarTimeSeries, PySeriesInterpolation};
#[allow(unused_imports)]
pub use surfaces::PyVolSurface;
#[allow(unused_imports)]
pub use term_structures::{
    PyBaseCorrelationCurve, PyCreditIndexData, PyDiscountCurve, PyForwardCurve, PyHazardCurve,
    PyInflationCurve,
};

use finstack_core::collections::HashSet;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "market_data")?;
    module.setattr(
        "__doc__",
        "Market data components mirroring finstack-core: curves, surfaces, FX, scalars, and aggregation context.",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let term_exports = term_structures::register(py, &module)?;
    exports.extend(term_exports.iter().copied());
    promote_exports(&module, "term_structures", &term_exports)?;

    let surface_exports = surfaces::register(py, &module)?;
    exports.extend(surface_exports.iter().copied());
    promote_exports(&module, "surfaces", &surface_exports)?;

    let scalar_exports = scalars::register(py, &module)?;
    exports.extend(scalar_exports.iter().copied());
    promote_exports(&module, "scalars", &scalar_exports)?;

    let dividend_exports = dividends::register(py, &module)?;
    exports.extend(dividend_exports.iter().copied());
    promote_exports(&module, "dividends", &dividend_exports)?;

    let fx_exports = fx::register(py, &module)?;
    exports.extend(fx_exports.iter().copied());
    promote_exports(&module, "fx", &fx_exports)?;

    let context_exports = context::register(py, &module)?;
    exports.extend(context_exports.iter().copied());
    promote_exports(&module, "context", &context_exports)?;

    let bump_exports = bumps::register(py, &module)?;
    exports.extend(bump_exports.iter().copied());
    promote_exports(&module, "bumps", &bump_exports)?;

    let diff_exports = diff::register(py, &module)?;
    exports.extend(diff_exports.iter().copied());
    promote_exports(&module, "diff", &diff_exports)?;

    let volatility_exports = volatility::register(py, &module)?;
    exports.extend(volatility_exports.iter().copied());
    promote_exports(&module, "volatility", &volatility_exports)?;

    let mut uniq = HashSet::default();
    exports.retain(|item| uniq.insert(*item));
    exports.sort_unstable();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

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
