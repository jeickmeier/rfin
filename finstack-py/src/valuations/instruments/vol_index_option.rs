//! Python bindings for VolatilityIndexOption.

use crate::core::dates::utils::py_to_date;
use crate::core::money::{extract_money, PyMoney};
use crate::errors::core_to_py;
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::vol_index_option::{
    VolIndexOptionSpecs, VolatilityIndexOption,
};
use finstack_valuations::instruments::{ExerciseStyle, OptionType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
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
    frozen
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

#[pymethods]
impl PyVolatilityIndexOption {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            strike,
            expiry,
            discount_curve,
            vol_index_curve,
            vol_of_vol_surface,
            *,
            option_type=None,
            exercise_style=None,
            multiplier=100.0,
            index_id="VIX"
        ),
        text_signature = "(cls, instrument_id, notional, strike, expiry, discount_curve, vol_index_curve, vol_of_vol_surface, /, *, option_type='call', exercise_style='european', multiplier=100.0, index_id='VIX')"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        strike: f64,
        expiry: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        vol_index_curve: Bound<'_, PyAny>,
        vol_of_vol_surface: Bound<'_, PyAny>,
        option_type: Option<&str>,
        exercise_style: Option<&str>,
        multiplier: Option<f64>,
        index_id: Option<&str>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let expiry_date = py_to_date(&expiry).context("expiry")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let vol_index_curve_id = CurveId::new(
            vol_index_curve
                .extract::<&str>()
                .context("vol_index_curve")?,
        );
        let vol_of_vol_surface_id = CurveId::new(
            vol_of_vol_surface
                .extract::<&str>()
                .context("vol_of_vol_surface")?,
        );

        let option_type_value = parse_option_type(option_type).context("option_type")?;
        let exercise_style_value =
            parse_exercise_style(exercise_style).context("exercise_style")?;

        let specs = VolIndexOptionSpecs {
            multiplier: multiplier.unwrap_or(100.0),
            index_id: index_id.unwrap_or("VIX").to_string(),
        };

        let option = VolatilityIndexOption::builder()
            .id(id)
            .notional(notional_money)
            .strike(strike)
            .expiry(expiry_date)
            .discount_curve_id(discount_curve_id)
            .vol_index_curve_id(vol_index_curve_id)
            .vol_of_vol_surface_id(vol_of_vol_surface_id)
            .option_type(option_type_value)
            .exercise_style(exercise_style_value)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map_err(core_to_py)?;

        Ok(Self::new(option))
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
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::VolatilityIndexOption)
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

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyVolatilityIndexOption>()?;
    Ok(vec!["VolatilityIndexOption"])
}
