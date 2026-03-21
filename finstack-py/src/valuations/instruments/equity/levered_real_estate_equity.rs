//! Python bindings for LeveredRealEstateEquity instrument.

use super::real_estate::PyRealEstateAsset;
use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::valuations::common::PyInstrumentType;
use crate::valuations::instruments::fixed_income::bond::PyBond;
use crate::valuations::instruments::fixed_income::convertible::PyConvertibleBond;
use crate::valuations::instruments::fixed_income::revolving_credit::PyRevolvingCredit;
use crate::valuations::instruments::fixed_income::term_loan::PyTermLoan;
use crate::valuations::instruments::rates::repo::PyRepo;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::real_estate::LeveredRealEstateEquity;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::InstrumentJson;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, Py};
use std::fmt;
use std::sync::Arc;

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LeveredRealEstateEquity",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLeveredRealEstateEquity {
    pub(crate) inner: Arc<LeveredRealEstateEquity>,
}

impl PyLeveredRealEstateEquity {
    pub(crate) fn new(inner: LeveredRealEstateEquity) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    fn parse_financing(list: &Bound<'_, PyList>) -> PyResult<Vec<InstrumentJson>> {
        let mut out = Vec::with_capacity(list.len());
        for item in list.iter() {
            // Allow passing an instrument JSON string (InstrumentJson itself) for maximum flexibility.
            if let Ok(s) = item.extract::<&str>() {
                let parsed: InstrumentJson = serde_json::from_str(s).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Failed to parse financing JSON: {e}"
                    ))
                })?;
                out.push(parsed);
                continue;
            }

            if let Ok(inst) = item.extract::<PyRef<'_, PyTermLoan>>() {
                out.push(InstrumentJson::TermLoan((*inst.inner).clone()));
                continue;
            }
            if let Ok(inst) = item.extract::<PyRef<'_, PyBond>>() {
                out.push(InstrumentJson::Bond((*inst.inner).clone()));
                continue;
            }
            if let Ok(inst) = item.extract::<PyRef<'_, PyConvertibleBond>>() {
                out.push(InstrumentJson::ConvertibleBond((*inst.inner).clone()));
                continue;
            }
            if let Ok(inst) = item.extract::<PyRef<'_, PyRevolvingCredit>>() {
                out.push(InstrumentJson::RevolvingCredit((*inst.inner).clone()));
                continue;
            }
            if let Ok(inst) = item.extract::<PyRef<'_, PyRepo>>() {
                out.push(InstrumentJson::Repo((*inst.inner).clone()));
                continue;
            }

            return Err(pyo3::exceptions::PyTypeError::new_err(
                "financing must contain TermLoan, Bond, ConvertibleBond, RevolvingCredit, Repo, or JSON strings",
            ));
        }
        Ok(out)
    }
}

#[pymethods]
impl PyLeveredRealEstateEquity {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, currency, asset, financing, discount_curve_id, exit_date=None)"
    )]
    #[pyo3(signature = (instrument_id, *, currency, asset, financing, discount_curve_id, exit_date=None))]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        asset: PyRef<'_, PyRealEstateAsset>,
        financing: Bound<'_, PyList>,
        discount_curve_id: &str,
        exit_date: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;

        let exit = if let Some(arg) = exit_date {
            if arg.is_none() {
                None
            } else {
                Some(py_to_date(&arg).context("exit_date")?)
            }
        } else {
            None
        };

        let financing = Self::parse_financing(&financing)?;

        let inst = LeveredRealEstateEquity::builder()
            .id(id)
            .currency(ccy)
            .asset((*asset.inner).clone())
            .financing(financing)
            .exit_date_opt(exit)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self::new(inst))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&*self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    #[getter]
    fn exit_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.exit_date {
            Some(d) => Ok(Some(date_to_py(py, d)?)),
            None => Ok(None),
        }
    }

    #[getter]
    fn asset(&self) -> PyRealEstateAsset {
        PyRealEstateAsset {
            inner: Arc::new(self.inner.asset.clone()),
        }
    }

    #[getter]
    fn financing_json(&self) -> PyResult<Vec<String>> {
        self.inner
            .financing
            .iter()
            .map(|i| {
                serde_json::to_string(i)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            })
            .collect()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "LeveredRealEstateEquity(id='{}', currency='{}')",
            self.inner.id.as_str(),
            self.inner.currency
        )
    }
}

impl fmt::Display for PyLeveredRealEstateEquity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LeveredRealEstateEquity({})", self.inner.id.as_str())
    }
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyLeveredRealEstateEquity>()?;
    Ok(())
}
