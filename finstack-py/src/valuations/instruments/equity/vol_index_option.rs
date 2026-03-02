//! Python bindings for VolatilityIndexOption.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::vol_index_option::{
    VolIndexOptionSpecs, VolatilityIndexOption,
};
use finstack_valuations::instruments::{ExerciseStyle, OptionType};
use finstack_valuations::prelude::Instrument;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

fn parse_option_type(label: Option<&str>) -> PyResult<OptionType> {
    match label {
        None => Ok(OptionType::Call),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "call" => Ok(OptionType::Call),
            "put" => Ok(OptionType::Put),
            _ => Err(PyValueError::new_err(format!(
                "Invalid option type: {}. Use 'call' or 'put'",
                s
            ))),
        },
    }
}

fn parse_exercise_style(label: Option<&str>) -> PyResult<ExerciseStyle> {
    match label {
        None => Ok(ExerciseStyle::European),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "european" => Ok(ExerciseStyle::European),
            "american" => Ok(ExerciseStyle::American),
            "bermudan" => Ok(ExerciseStyle::Bermudan),
            _ => Err(PyValueError::new_err(format!(
                "Invalid exercise style: {}. Use 'european', 'american', or 'bermudan'",
                s
            ))),
        },
    }
}

/// Volatility index option wrapper (e.g., VIX options).
///
/// Parameters
/// ----------
/// instrument_id : str
///     Unique identifier for the instrument.
/// notional : Money
///     Notional amount (e.g., $100,000 USD).
/// strike : float
///     Strike price (e.g., 20.0 for VIX at 20).
/// expiry : date
///     Expiry date of the option.
/// discount_curve : str
///     ID of the discount curve for NPV calculations.
/// vol_index_curve : str
///     ID of the volatility index curve for forward levels.
/// vol_of_vol_surface : str
///     ID of the volatility-of-volatility surface.
/// option_type : str, optional
///     Option type: "call" (default) or "put".
/// exercise_style : str, optional
///     Exercise style: "european" (default), "american", or "bermudan".
/// multiplier : float, optional
///     Contract multiplier (default: 100 for VIX options).
/// index_id : str, optional
///     Index identifier (default: "VIX").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolatilityIndexOption",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolatilityIndexOption {
    pub(crate) inner: Arc<VolatilityIndexOption>,
}

impl PyVolatilityIndexOption {
    pub(crate) fn new(inner: VolatilityIndexOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolatilityIndexOptionBuilder",
    unsendable
)]
pub struct PyVolatilityIndexOptionBuilder {
    instrument_id: InstrumentId,
    pending_notional_amount: Option<f64>,
    pending_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    expiry: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    vol_index_curve_id: Option<CurveId>,
    vol_of_vol_surface_id: Option<CurveId>,
    option_type: OptionType,
    exercise_style: ExerciseStyle,
    multiplier: f64,
    index_id: String,
}

impl PyVolatilityIndexOptionBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_notional_amount: None,
            pending_currency: None,
            strike: None,
            expiry: None,
            discount_curve_id: None,
            vol_index_curve_id: None,
            vol_of_vol_surface_id: None,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            multiplier: 100.0,
            index_id: "VIX".to_string(),
        }
    }

    fn notional_money(&self) -> Option<Money> {
        match (self.pending_notional_amount, self.pending_currency) {
            (Some(amount), Some(currency)) => Some(Money::new(amount, currency)),
            _ => None,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.notional_money().is_none() {
            return Err(PyValueError::new_err(
                "Both notional() and currency() must be provided before build().",
            ));
        }
        if self.strike.is_none() {
            return Err(PyValueError::new_err("strike() is required."));
        }
        if self.expiry.is_none() {
            return Err(PyValueError::new_err("expiry() is required."));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err("disc_id() is required."));
        }
        if self.vol_index_curve_id.is_none() {
            return Err(PyValueError::new_err("vol_index_curve_id() is required."));
        }
        if self.vol_of_vol_surface_id.is_none() {
            return Err(PyValueError::new_err(
                "vol_of_vol_surface_id() is required.",
            ));
        }
        Ok(())
    }

    fn parse_currency(value: &Bound<'_, PyAny>) -> PyResult<finstack_core::currency::Currency> {
        if let Ok(py_ccy) = value.extract::<PyRef<PyCurrency>>() {
            Ok(py_ccy.inner)
        } else if let Ok(code) = value.extract::<&str>() {
            code.parse::<finstack_core::currency::Currency>()
                .map_err(|_| PyValueError::new_err("Invalid currency code"))
        } else {
            Err(PyTypeError::new_err("currency() expects str or Currency"))
        }
    }
}

#[pymethods]
impl PyVolatilityIndexOptionBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn notional(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyResult<PyRefMut<'_, Self>> {
        if amount <= 0.0 {
            return Err(PyValueError::new_err("notional must be positive"));
        }
        slf.pending_notional_amount = Some(amount);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.pending_currency = Some(Self::parse_currency(currency)?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, money)")]
    fn money<'py>(mut slf: PyRefMut<'py, Self>, money: PyRef<'py, PyMoney>) -> PyRefMut<'py, Self> {
        slf.pending_notional_amount = Some(money.inner.amount());
        slf.pending_currency = Some(money.inner.currency());
        slf
    }

    #[pyo3(text_signature = "($self, strike)")]
    fn strike(mut slf: PyRefMut<'_, Self>, strike: f64) -> PyRefMut<'_, Self> {
        slf.strike = Some(strike);
        slf
    }

    #[pyo3(text_signature = "($self, expiry)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        expiry: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.expiry = Some(py_to_date(&expiry).context("expiry")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn vol_index_curve_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.vol_index_curve_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn vol_of_vol_surface_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.vol_of_vol_surface_id = Some(CurveId::new(curve_id.as_str()));
        slf
    }

    #[pyo3(text_signature = "($self, option_type)")]
    fn option_type(
        mut slf: PyRefMut<'_, Self>,
        option_type: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.option_type = parse_option_type(Some(option_type.as_str()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, exercise_style)")]
    fn exercise_style(
        mut slf: PyRefMut<'_, Self>,
        exercise_style: String,
    ) -> PyResult<PyRefMut<'_, Self>> {
        slf.exercise_style = parse_exercise_style(Some(exercise_style.as_str()))?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, multiplier)")]
    fn multiplier(mut slf: PyRefMut<'_, Self>, multiplier: f64) -> PyRefMut<'_, Self> {
        slf.multiplier = multiplier;
        slf
    }

    #[pyo3(text_signature = "($self, index_id)")]
    fn index_id(mut slf: PyRefMut<'_, Self>, index_id: String) -> PyRefMut<'_, Self> {
        slf.index_id = index_id;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyVolatilityIndexOption> {
        slf.ensure_ready()?;
        let notional = slf.notional_money().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing notional after validation",
            )
        })?;
        let strike = slf.strike.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing strike after validation",
            )
        })?;
        let expiry = slf.expiry.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing expiry after validation",
            )
        })?;
        let discount_curve_id = slf.discount_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing discount curve after validation",
            )
        })?;
        let vol_index_curve_id = slf.vol_index_curve_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing vol index curve after validation",
            )
        })?;
        let vol_of_vol_surface_id = slf.vol_of_vol_surface_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "VolatilityIndexOptionBuilder internal error: missing vol-of-vol surface after validation",
            )
        })?;

        let specs = VolIndexOptionSpecs {
            multiplier: slf.multiplier,
            index_id: slf.index_id.clone(),
        };

        let option = VolatilityIndexOption::builder()
            .id(slf.instrument_id.clone())
            .notional(notional)
            .strike(strike)
            .expiry(expiry)
            .discount_curve_id(discount_curve_id)
            .vol_index_curve_id(vol_index_curve_id)
            .vol_of_vol_surface_id(vol_of_vol_surface_id)
            .option_type(slf.option_type)
            .exercise_style(slf.exercise_style)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        Ok(PyVolatilityIndexOption::new(option))
    }

    fn __repr__(&self) -> String {
        "VolatilityIndexOptionBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyVolatilityIndexOption {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    /// Start a fluent builder (builder-only API).
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyVolatilityIndexOptionBuilder>> {
        let py = cls.py();
        let builder = PyVolatilityIndexOptionBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[getter]
    fn option_type(&self) -> &str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn exercise_style(&self) -> &'static str {
        match self.inner.exercise_style {
            ExerciseStyle::European => "european",
            ExerciseStyle::American => "american",
            ExerciseStyle::Bermudan => "bermudan",
        }
    }

    #[getter]
    fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_index_curve_id(&self) -> String {
        self.inner.vol_index_curve_id.as_str().to_string()
    }

    #[getter]
    fn vol_of_vol_surface_id(&self) -> String {
        self.inner.vol_of_vol_surface_id.as_str().to_string()
    }

    /// Contract multiplier.
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.contract_specs.multiplier
    }

    /// Volatility index identifier.
    #[getter]
    fn index_id_spec(&self) -> &str {
        &self.inner.contract_specs.index_id
    }

    /// Contract specifications as a structured object.
    #[getter]
    fn contract_specs(&self) -> PyVolIndexOptionSpecs {
        PyVolIndexOptionSpecs {
            inner: VolIndexOptionSpecs {
                multiplier: self.inner.contract_specs.multiplier,
                index_id: self.inner.contract_specs.index_id.clone(),
            },
        }
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::VolatilityIndexOption)
    }

    #[pyo3(signature = (market, as_of))]
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, date))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    #[pyo3(signature = (market, as_of))]
    fn npv_raw(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.npv_raw(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn forward_vol(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.forward_vol(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn delta(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.delta(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn gamma(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.gamma(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn vega(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.vega(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn theta(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.theta(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn intrinsic_value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.intrinsic_value(&market.inner, date))
            .map_err(core_to_py)
    }

    #[pyo3(signature = (market, as_of))]
    fn time_value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<f64> {
        let date = py_to_date(&as_of)?;
        py.detach(|| self.inner.time_value(&market.inner, date))
            .map_err(core_to_py)
    }

    fn num_contracts(&self) -> f64 {
        self.inner.num_contracts()
    }

    fn black_price(&self, forward: f64, sigma: f64, t: f64) -> f64 {
        self.inner.black_price(forward, sigma, t)
    }

    fn __repr__(&self) -> PyResult<String> {
        let opt_type = match self.inner.option_type {
            OptionType::Call => "Call",
            OptionType::Put => "Put",
        };
        Ok(format!(
            "VolatilityIndexOption(id='{}', strike={:.2}, type={})",
            self.inner.id, self.inner.strike, opt_type
        ))
    }
}

impl fmt::Display for PyVolatilityIndexOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opt_type = match self.inner.option_type {
            OptionType::Call => "Call",
            OptionType::Put => "Put",
        };
        write!(
            f,
            "VolatilityIndexOption({}, strike={:.2}, type={})",
            self.inner.id, self.inner.strike, opt_type
        )
    }
}

// ============================================================================
// OPTION SPECS WRAPPER
// ============================================================================

/// Volatility index option specifications.
///
/// Parameters
/// ----------
/// multiplier : float
///     Contract multiplier (e.g., 100 for VIX options).
/// index_id : str
///     Volatility index identifier (e.g., "VIX").
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "VolIndexOptionSpecs",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolIndexOptionSpecs {
    pub(crate) inner: VolIndexOptionSpecs,
}

#[pymethods]
impl PyVolIndexOptionSpecs {
    #[new]
    #[pyo3(text_signature = "(multiplier, index_id)")]
    fn new_py(multiplier: f64, index_id: String) -> Self {
        Self {
            inner: VolIndexOptionSpecs {
                multiplier,
                index_id,
            },
        }
    }

    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    #[getter]
    fn index_id(&self) -> &str {
        &self.inner.index_id
    }

    fn __repr__(&self) -> String {
        format!(
            "VolIndexOptionSpecs(multiplier={}, index_id='{}')",
            self.inner.multiplier, self.inner.index_id
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyVolatilityIndexOption>()?;
    module.add_class::<PyVolatilityIndexOptionBuilder>()?;
    module.add_class::<PyVolIndexOptionSpecs>()?;
    Ok(vec![
        "VolatilityIndexOption",
        "VolatilityIndexOptionBuilder",
        "VolIndexOptionSpecs",
    ])
}
