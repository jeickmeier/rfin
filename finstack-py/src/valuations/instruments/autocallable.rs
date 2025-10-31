use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::py_to_date;
use crate::valuations::common::{extract_curve_id, extract_instrument_id};
use finstack_valuations::instruments::autocallable::{Autocallable, FinalPayoffType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

/// Autocallable structured product instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Autocallable",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyAutocallable {
    pub(crate) inner: Autocallable,
}

impl PyAutocallable {
    pub(crate) fn new(inner: Autocallable) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAutocallable {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, ticker, observation_dates, autocall_barriers, coupons, final_barrier, final_payoff_type, participation_rate, cap_level, notional, discount_curve, spot_id, vol_surface, *, dividend_yield_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create an autocallable structured product.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     ticker: Equity ticker symbol for the underlying asset.
    ///     observation_dates: List of observation dates for autocall checks.
    ///     autocall_barriers: List of barrier ratios (relative to initial spot) for each observation.
    ///     coupons: List of coupon rates for each observation date.
    ///     final_barrier: Final barrier ratio for terminal payoff.
    ///     final_payoff_type: Final payoff type dict with 'type' and optional params.
    ///     participation_rate: Participation rate for final payoff.
    ///     cap_level: Cap level ratio for final payoff.
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     spot_id: Spot price identifier.
    ///     vol_surface: Volatility surface identifier.
    ///     dividend_yield_id: Optional dividend yield identifier.
    ///
    /// Returns:
    ///     Autocallable: Configured autocallable instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        ticker: &str,
        observation_dates: Bound<'_, PyList>,
        autocall_barriers: Bound<'_, PyList>,
        coupons: Bound<'_, PyList>,
        final_barrier: f64,
        final_payoff_type: Bound<'_, PyAny>,
        participation_rate: f64,
        cap_level: f64,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        spot_id: &str,
        vol_surface: Bound<'_, PyAny>,
        dividend_yield_id: Option<&str>,
    ) -> PyResult<Self> {
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let vol_id = extract_curve_id(&vol_surface)?;

        // Parse observation dates
        let mut obs_dates = Vec::new();
        for item in observation_dates.iter() {
            obs_dates.push(py_to_date(&item)?);
        }

        // Parse autocall barriers
        let mut barriers = Vec::new();
        for item in autocall_barriers.iter() {
            barriers.push(item.extract::<f64>()?);
        }

        // Parse coupons
        let mut coupon_rates = Vec::new();
        for item in coupons.iter() {
            coupon_rates.push(item.extract::<f64>()?);
        }

        // Parse final payoff type from dict or string
        let payoff_type = if let Ok(dict) = final_payoff_type.downcast::<pyo3::types::PyDict>() {
            let py_type_val = dict
                .get_item("type")?
                .ok_or_else(|| PyValueError::new_err("Missing 'type' key in final_payoff_type"))?;
            let py_type = py_type_val.extract::<&str>()?;

            match py_type.to_lowercase().as_str() {
                "capital_protection" => {
                    let floor = dict
                        .get_item("floor")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'floor' for capital_protection"))?
                        .extract::<f64>()?;
                    FinalPayoffType::CapitalProtection { floor }
                }
                "participation" => {
                    let rate = dict
                        .get_item("rate")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'rate' for participation"))?
                        .extract::<f64>()?;
                    FinalPayoffType::Participation { rate }
                }
                "knock_in_put" => {
                    let strike = dict
                        .get_item("strike")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'strike' for knock_in_put"))?
                        .extract::<f64>()?;
                    FinalPayoffType::KnockInPut { strike }
                }
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown final payoff type: {other}"
                    )))
                }
            }
        } else if let Ok(py_type) = final_payoff_type.extract::<&str>() {
            match py_type.to_lowercase().as_str() {
                "capital_protection" => {
                    return Err(PyValueError::new_err(
                        "capital_protection requires dict with 'floor'"
                    ))
                }
                "participation" => {
                    return Err(PyValueError::new_err(
                        "participation requires dict with 'rate'"
                    ))
                }
                "knock_in_put" => {
                    return Err(PyValueError::new_err(
                        "knock_in_put requires dict with 'strike'"
                    ))
                }
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown final payoff type: {other}"
                    )))
                }
            }
        } else {
            return Err(PyValueError::new_err(
                "final_payoff_type must be a dict with 'type' key"
            ));
        };

        let mut builder = Autocallable::builder();
        builder = builder.id(id);
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.observation_dates(obs_dates);
        builder = builder.autocall_barriers(barriers);
        builder = builder.coupons(coupon_rates);
        builder = builder.final_barrier(final_barrier);
        builder = builder.final_payoff_type(payoff_type);
        builder = builder.participation_rate(participation_rate);
        builder = builder.cap_level(cap_level);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.disc_id(disc_id);
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_id(vol_id);
        if let Some(div) = dividend_yield_id {
            builder = builder.div_yield_id(div.to_string());
        }
        let autocallable = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build Autocallable: {e}"))
        })?;
        Ok(Self::new(autocallable))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Underlying ticker symbol.
    #[getter]
    fn ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    /// Final barrier ratio.
    #[getter]
    fn final_barrier(&self) -> f64 {
        self.inner.final_barrier
    }

    /// Participation rate.
    #[getter]
    fn participation_rate(&self) -> f64 {
        self.inner.participation_rate
    }

    /// Cap level.
    #[getter]
    fn cap_level(&self) -> f64 {
        self.inner.cap_level
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    fn __repr__(&self) -> String {
        format!(
            "Autocallable(id='{}', ticker='{}', final_barrier={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.final_barrier
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "autocallable")?;
    module.add_class::<PyAutocallable>()?;
    let exports = ["Autocallable"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}

