use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::common::parameters::OptionType;
use finstack_valuations::instruments::swaption::parameters::SwaptionParams;
use finstack_valuations::instruments::swaption::Swaption;
use finstack_valuations::instruments::swaption::{SwaptionExercise, SwaptionSettlement};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_settlement(label: Option<&str>) -> PyResult<SwaptionSettlement> {
    match label {
        None => Ok(SwaptionSettlement::Physical),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

fn parse_exercise(label: Option<&str>) -> PyResult<SwaptionExercise> {
    match label {
        None => Ok(SwaptionExercise::European),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

/// Swaption bindings with payer/receiver constructors.
#[pyclass(module = "finstack.valuations.instruments", name = "Swaption", frozen)]
#[derive(Clone, Debug)]
pub struct PySwaption {
    pub(crate) inner: Swaption,
}

impl PySwaption {
    pub(crate) fn new(inner: Swaption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySwaption {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, strike, expiry, swap_start, swap_end, discount_curve, forward_curve, vol_surface, exercise='european', settlement='physical')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a payer swaption (pay fixed underlying swap).
    fn payer(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        exercise: Option<&str>,
        settlement: Option<&str>,
    ) -> PyResult<Self> {
        construct_swaption(
            instrument_id,
            notional,
            strike,
            expiry,
            swap_start,
            swap_end,
            discount_curve,
            forward_curve,
            vol_surface,
            exercise,
            settlement,
            true,
        )
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, strike, expiry, swap_start, swap_end, discount_curve, forward_curve, vol_surface, exercise='european', settlement='physical')"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a receiver swaption (receive fixed underlying swap).
    fn receiver(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        swap_start: Bound<'_, PyAny>,
        swap_end: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        exercise: Option<&str>,
        settlement: Option<&str>,
    ) -> PyResult<Self> {
        construct_swaption(
            instrument_id,
            notional,
            strike,
            expiry,
            swap_start,
            swap_end,
            discount_curve,
            forward_curve,
            vol_surface,
            exercise,
            settlement,
            false,
        )
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
        self.inner.strike_rate
    }

    #[getter]
    fn expiry(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.expiry)
    }

    #[getter]
    fn swap_start(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.swap_start)
    }

    #[getter]
    fn swap_end(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.swap_end)
    }

    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "payer",
            OptionType::Put => "receiver",
        }
    }

    #[getter]
    fn settlement(&self) -> &'static str {
        match self.inner.settlement {
            SwaptionSettlement::Physical => "physical",
            SwaptionSettlement::Cash => "cash",
        }
    }

    #[getter]
    fn exercise(&self) -> &'static str {
        match self.inner.exercise {
            SwaptionExercise::European => "european",
            SwaptionExercise::Bermudan => "bermudan",
            SwaptionExercise::American => "american",
        }
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    #[getter]
    fn vol_surface(&self) -> &str {
        self.inner.vol_id.as_str()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Swaption)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Swaption(id='{}', type='{}')",
            self.inner.id,
            self.option_type()
        ))
    }
}

impl fmt::Display for PySwaption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Swaption({}, type={})",
            self.inner.id,
            self.option_type()
        )
    }
}

fn construct_swaption(
    instrument_id: Bound<'_, PyAny>,
    notional: Bound<'_, PyAny>,
    strike: f64,
    expiry: Bound<'_, PyAny>,
    swap_start: Bound<'_, PyAny>,
    swap_end: Bound<'_, PyAny>,
    discount_curve: Bound<'_, PyAny>,
    forward_curve: Bound<'_, PyAny>,
    vol_surface: Bound<'_, PyAny>,
    exercise: Option<&str>,
    settlement: Option<&str>,
    payer: bool,
) -> PyResult<PySwaption> {
    let id = extract_instrument_id(&instrument_id)?;
    let amt = extract_money(&notional)?;
    let expiry_date = py_to_date(&expiry)?;
    let start = py_to_date(&swap_start)?;
    let end = py_to_date(&swap_end)?;
    let disc = extract_curve_id(&discount_curve)?;
    let fwd = extract_curve_id(&forward_curve)?;
    let exercise_style = parse_exercise(exercise)?;
    let settlement_type = parse_settlement(settlement)?;

    let params = if payer {
        SwaptionParams::payer(amt, strike, expiry_date, start, end)
    } else {
        SwaptionParams::receiver(amt, strike, expiry_date, start, end)
    };

    let vol_id = extract_curve_id(&vol_surface)?;

    let mut swaption = if payer {
        Swaption::new_payer(id.clone(), &params, disc, fwd, vol_id)
    } else {
        Swaption::new_receiver(id.clone(), &params, disc, fwd, vol_id)
    };

    swaption.exercise = exercise_style;
    swaption.settlement = settlement_type;
    if !payer {
        swaption.option_type = OptionType::Put;
    }

    Ok(PySwaption::new(swaption))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySwaption>()?;
    Ok(vec!["Swaption"])
}
