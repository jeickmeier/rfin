//! Python wrappers for CDS-family instruments.

use super::{parse_date, PyValuationResult};
use crate::bindings::extract::extract_market;
use crate::errors::display_to_py;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_valuations::instruments::credit_derivatives::cds_tranche::CDSTranche;
use finstack_valuations::instruments::{Instrument, InstrumentJson};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule};

macro_rules! credit_derivative_wrapper {
    ($py_name:literal, $py_struct:ident, $rust_ty:ty, $variant:ident, $model:literal, $example:expr) => {
        #[pyclass(name = $py_name, module = "finstack.valuations.credit_derivatives", skip_from_py_object)]
        #[derive(Clone)]
        struct $py_struct {
            inner: $rust_ty,
        }

        #[pymethods]
        impl $py_struct {
            #[staticmethod]
            fn example() -> PyResult<Self> {
                Ok(Self { inner: $example.map_err(display_to_py)? })
            }

            #[staticmethod]
            fn from_json(json: &str) -> PyResult<Self> {
                if let Ok(inner) = serde_json::from_str::<$rust_ty>(json) {
                    return Ok(Self { inner });
                }
                match serde_json::from_str::<InstrumentJson>(json).map_err(display_to_py)? {
                    InstrumentJson::$variant(inner) => Ok(Self { inner }),
                    _ => Err(display_to_py(format!("JSON is not a {}", $py_name))),
                }
            }

            fn to_json(&self) -> PyResult<String> {
                serde_json::to_string_pretty(&InstrumentJson::$variant(self.inner.clone()))
                    .map_err(display_to_py)
            }

            fn validate(&self) -> PyResult<()> {
                self.inner.validate().map_err(display_to_py)
            }

            fn price(&self, market: &Bound<'_, PyAny>, as_of: &str) -> PyResult<PyValuationResult> {
                let market = extract_market(market)?;
                let result = finstack_valuations::pricer::standard_registry()
                    .price_with_metrics(
                        &self.inner,
                        finstack_valuations::pricer::parse_model_key($model).map_err(display_to_py)?,
                        &market,
                        parse_date(as_of)?,
                        &[],
                        Default::default(),
                    )
                    .map_err(display_to_py)?;
                Ok(PyValuationResult { inner: result })
            }
        }
    };
}

credit_derivative_wrapper!(
    "CreditDefaultSwap",
    PyCreditDefaultSwap,
    CreditDefaultSwap,
    CreditDefaultSwap,
    "hazard_rate",
    Ok::<CreditDefaultSwap, finstack_core::Error>(CreditDefaultSwap::example())
);

credit_derivative_wrapper!(
    "CDSIndex",
    PyCDSIndex,
    CDSIndex,
    CDSIndex,
    "hazard_rate",
    Ok::<CDSIndex, finstack_core::Error>(CDSIndex::example())
);

credit_derivative_wrapper!(
    "CDSTranche",
    PyCDSTranche,
    CDSTranche,
    CDSTranche,
    "hazard_rate",
    Ok::<CDSTranche, finstack_core::Error>(CDSTranche::example())
);

credit_derivative_wrapper!(
    "CDSOption",
    PyCDSOption,
    CDSOption,
    CDSOption,
    "black76",
    CDSOption::example()
);

pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "credit_derivatives")?;
    module.add_class::<PyCreditDefaultSwap>()?;
    module.add_class::<PyCDSIndex>()?;
    module.add_class::<PyCDSTranche>()?;
    module.add_class::<PyCDSOption>()?;
    let all = PyList::new(
        py,
        ["CreditDefaultSwap", "CDSIndex", "CDSTranche", "CDSOption"],
    )?;
    module.setattr("__all__", all)?;
    parent.add_submodule(&module)?;
    parent.setattr("credit_derivatives", &module)?;
    Ok(())
}
