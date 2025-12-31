use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::{to_optional_string, PyInstrumentType};
use finstack_core::dates::BusinessDayConvention;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::repo::{CollateralSpec, CollateralType, Repo, RepoType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_repo_type(label: Option<&str>) -> PyResult<RepoType> {
    match label {
        None => Ok(RepoType::Term),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Collateral specification helper mirroring `CollateralSpec`.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RepoCollateral",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRepoCollateral {
    pub(crate) inner: CollateralSpec,
}

#[pymethods]
impl PyRepoCollateral {
    #[new]
    #[pyo3(
        signature = (
            instrument_id,
            quantity,
            market_value_id,
            *,
            collateral_type = "general",
            special_security_id = None,
            special_rate_adjust_bp = None
        ),
        text_signature = "(instrument_id, quantity, market_value_id, *, collateral_type='general', special_security_id=None, special_rate_adjust_bp=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        instrument_id: &str,
        quantity: f64,
        market_value_id: &str,
        collateral_type: Option<&str>,
        special_security_id: Option<&str>,
        special_rate_adjust_bp: Option<f64>,
    ) -> PyResult<Self> {
        let ctype = match collateral_type
            .map(crate::core::common::labels::normalize_label)
            .as_deref()
        {
            None | Some("general") => CollateralType::General,
            Some("special") => CollateralType::Special {
                security_id: special_security_id.map(|s| s.to_string()).ok_or_else(|| {
                    PyValueError::new_err("special_security_id required for special collateral")
                })?,
                rate_adjustment_bp: special_rate_adjust_bp,
            },
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Unknown collateral_type: {other}",
                )))
            }
        };
        let spec = CollateralSpec {
            collateral_type: ctype,
            instrument_id: instrument_id.to_string(),
            quantity,
            market_value_id: market_value_id.to_string(),
        };
        Ok(Self { inner: spec })
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[getter]
    fn market_value_id(&self) -> &str {
        &self.inner.market_value_id
    }
}

/// Repo wrapper exposing a convenience constructor.
#[pyclass(module = "finstack.valuations.instruments", name = "Repo", frozen)]
#[derive(Clone, Debug)]
pub struct PyRepo {
    pub(crate) inner: Repo,
}

impl PyRepo {
    pub(crate) fn new(inner: Repo) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRepo {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            cash_amount,
            collateral,
            repo_rate,
            start_date,
            maturity,
            discount_curve,
            *,
            repo_type = "term",
            haircut = 0.0,
            day_count = None,
            business_day_convention = None,
            calendar = None,
            triparty = false
        ),
        text_signature = "(cls, instrument_id, cash_amount, collateral, repo_rate, start_date, maturity, discount_curve, *, repo_type='term', haircut=0.0, day_count='act_360', business_day_convention='following', calendar=None, triparty=False)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        cash_amount: Bound<'_, PyAny>,
        collateral: PyRepoCollateral,
        repo_rate: f64,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        repo_type: Option<&str>,
        haircut: Option<f64>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        calendar: Option<&str>,
        triparty: Option<bool>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let cash = extract_money(&cash_amount).context("cash_amount")?;
        let start = py_to_date(&start_date).context("start_date")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let repo_type_value = parse_repo_type(repo_type).context("repo_type")?;
        let day_count_value = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract().context("day_count")?;
            value
        } else {
            finstack_core::dates::DayCount::Act360
        };
        let bdc_value = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) =
                obj.extract().context("business_day_convention")?;
            value
        } else {
            BusinessDayConvention::Following
        };

        let mut builder = Repo::builder();
        builder = builder.id(id);
        builder = builder.cash_amount(cash);
        builder = builder.collateral(collateral.inner.clone());
        builder = builder.repo_rate(repo_rate);
        builder = builder.start_date(start);
        builder = builder.maturity(maturity_date);
        builder = builder.haircut(haircut.unwrap_or(0.0));
        builder = builder.repo_type(repo_type_value);
        builder = builder.triparty(triparty.unwrap_or(false));
        builder = builder.day_count(day_count_value);
        builder = builder.bdc(bdc_value);
        builder = builder.calendar_id_opt(to_optional_string(calendar));
        builder = builder.discount_curve_id(discount_curve_id);

        let repo = builder.build().map_err(core_to_py)?;
        Ok(Self::new(repo))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn cash_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.cash_amount)
    }

    #[getter]
    fn repo_rate(&self) -> f64 {
        self.inner.repo_rate
    }

    #[getter]
    fn start_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.start_date)
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Repo)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Repo(id='{}', rate={:.4})",
            self.inner.id, self.inner.repo_rate
        ))
    }
}

impl fmt::Display for PyRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repo({}, rate={:.4})",
            self.inner.id, self.inner.repo_rate
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyRepoCollateral>()?;
    module.add_class::<PyRepo>()?;
    Ok(vec!["RepoCollateral", "Repo"])
}
