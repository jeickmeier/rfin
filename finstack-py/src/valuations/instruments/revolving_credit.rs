use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, parse_frequency_label};
use finstack_valuations::instruments::revolving_credit::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
    StochasticUtilizationSpec, UtilizationProcess,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

/// Revolving credit facility instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RevolvingCredit",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRevolvingCredit {
    pub(crate) inner: RevolvingCredit,
}

impl PyRevolvingCredit {
    pub(crate) fn new(inner: RevolvingCredit) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRevolvingCredit {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, commitment_amount, drawn_amount, commitment_date, maturity_date, base_rate_spec, payment_frequency, fees, draw_repay_spec, discount_curve)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a revolving credit facility.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     commitment_amount: Total committed amount as :class:`finstack.core.money.Money`.
    ///     drawn_amount: Initial drawn amount as :class:`finstack.core.money.Money`.
    ///     commitment_date: Date when facility becomes available.
    ///     maturity_date: Date when facility expires.
    ///     base_rate_spec: Base rate specification (dict with 'type' and params).
    ///     payment_frequency: Payment frequency (e.g., 'quarterly').
    ///     fees: Fee structure dict.
    ///     draw_repay_spec: Draw/repayment specification (dict).
    ///     discount_curve: Discount curve identifier.
    ///
    /// Returns:
    ///     RevolvingCredit: Configured revolving credit instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        commitment_amount: Bound<'_, PyAny>,
        drawn_amount: Bound<'_, PyAny>,
        commitment_date: Bound<'_, PyAny>,
        maturity_date: Bound<'_, PyAny>,
        base_rate_spec: Bound<'_, PyAny>,
        payment_frequency: Option<&str>,
        fees: Bound<'_, PyAny>,
        draw_repay_spec: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let commitment = extract_money(&commitment_amount)?;
        let drawn = extract_money(&drawn_amount)?;
        let commit_date = py_to_date(&commitment_date)?;
        let mat_date = py_to_date(&maturity_date)?;
        let disc_id = extract_curve_id(&discount_curve)?;

        // Parse base rate spec
        let base_rate = if let Ok(dict) = base_rate_spec.downcast::<PyDict>() {
            let py_type_item = dict
                .get_item("type")?
                .ok_or_else(|| PyValueError::new_err("Missing 'type' key in base_rate_spec"))?;
            let py_type = py_type_item.extract::<String>()?;

            match py_type.to_lowercase().as_str() {
                "fixed" => {
                    let rate = dict
                        .get_item("rate")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'rate' for fixed rate"))?
                        .extract::<f64>()?;
                    BaseRateSpec::Fixed { rate }
                }
                "floating" => {
                    let index_id_item = dict
                        .get_item("index_id")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'index_id' for floating rate"))?;
                    let index_id_str = index_id_item.extract::<String>()?;
                    let margin_bp = dict
                        .get_item("margin_bp")?
                        .and_then(|v| v.extract::<f64>().ok())
                        .unwrap_or(0.0);
                    let reset_freq_str = dict
                        .get_item("reset_freq")?
                        .and_then(|v| v.extract::<String>().ok());
                    let reset_freq = parse_frequency(reset_freq_str.as_deref())?;
                    BaseRateSpec::Floating {
                        index_id: finstack_core::types::CurveId::new(&index_id_str),
                        margin_bp,
                        reset_freq,
                    }
                }
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown base rate type: {other}"
                    )))
                }
            }
        } else {
            return Err(PyValueError::new_err(
                "base_rate_spec must be a dict with 'type' key"
            ));
        };

        // Parse payment frequency
        let pay_freq = parse_frequency(payment_frequency)?;

        // Parse fees
        let fees_struct = if let Ok(dict) = fees.downcast::<PyDict>() {
            RevolvingCreditFees {
                upfront_fee: dict
                    .get_item("upfront_fee")?
                    .and_then(|v| extract_money(&v).ok()),
                commitment_fee_bp: dict
                    .get_item("commitment_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
                usage_fee_bp: dict
                    .get_item("usage_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
                facility_fee_bp: dict
                    .get_item("facility_fee_bp")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .unwrap_or(0.0),
            }
        } else {
            RevolvingCreditFees::default()
        };

        // Parse draw/repay spec
        let draw_repay = if let Ok(dict) = draw_repay_spec.downcast::<PyDict>() {
            if let Ok(Some(deterministic)) = dict.get_item("deterministic") {
                let events_list = deterministic
                    .downcast::<PyList>()
                    .map_err(|_| PyValueError::new_err("deterministic must be a list"))?;
                let mut events = Vec::new();
                for item in events_list.iter() {
                    let event_dict = item.downcast::<PyDict>()?;
                    let date = py_to_date(
                        &event_dict
                            .get_item("date")?
                            .ok_or_else(|| PyValueError::new_err("Missing 'date' in event"))?,
                    )?;
                    let amount = extract_money(
                        &event_dict
                            .get_item("amount")?
                            .ok_or_else(|| PyValueError::new_err("Missing 'amount' in event"))?,
                    )?;
                    let is_draw = event_dict
                        .get_item("is_draw")?
                        .and_then(|v| v.extract::<bool>().ok())
                        .unwrap_or(true);
                    events.push(DrawRepayEvent {
                        date,
                        amount,
                        is_draw,
                    });
                }
                DrawRepaySpec::Deterministic(events)
            } else if let Ok(Some(stochastic)) = dict.get_item("stochastic") {
                let stoch_dict = stochastic
                    .downcast::<PyDict>()
                    .map_err(|_| PyValueError::new_err("stochastic must be a dict"))?;
                let process_dict_item = stoch_dict
                    .get_item("utilization_process")?
                    .ok_or_else(|| PyValueError::new_err("Missing 'utilization_process'"))?;
                let process_dict = process_dict_item.downcast::<PyDict>()?;
                let process_type_val = process_dict
                    .get_item("type")?
                    .ok_or_else(|| PyValueError::new_err("Missing 'type' in utilization_process"))?;
                let process_type = process_type_val.extract::<String>()?;

                let utilization_process = match process_type.to_lowercase().as_str() {
                    "mean_reverting" | "meanreverting" => {
                        let target_rate = process_dict
                            .get_item("target_rate")?
                            .ok_or_else(|| PyValueError::new_err("Missing 'target_rate'"))?
                            .extract::<f64>()?;
                        let speed = process_dict
                            .get_item("speed")?
                            .ok_or_else(|| PyValueError::new_err("Missing 'speed'"))?
                            .extract::<f64>()?;
                        let volatility = process_dict
                            .get_item("volatility")?
                            .ok_or_else(|| PyValueError::new_err("Missing 'volatility'"))?
                            .extract::<f64>()?;
                        UtilizationProcess::MeanReverting {
                            target_rate,
                            speed,
                            volatility,
                        }
                    }
                    other => {
                        return Err(PyValueError::new_err(format!(
                            "Unknown utilization process: {other}"
                        )))
                    }
                };

                let num_paths = stoch_dict
                    .get_item("num_paths")?
                    .ok_or_else(|| PyValueError::new_err("Missing 'num_paths'"))?
                    .extract::<usize>()?;
                let seed = stoch_dict
                    .get_item("seed")?
                    .and_then(|v| v.extract::<Option<u64>>().ok())
                    .flatten();

                // Construct StochasticUtilizationSpec
                // When --all-features is used (as in make lint), the mc feature in
                // finstack-valuations is enabled via transitive dependency features,
                // so mc_config field exists and must be provided.
                // Note: This code will only compile when mc feature is enabled in finstack-valuations.
                #[allow(unused_attributes)]
                let spec = StochasticUtilizationSpec {
                    utilization_process,
                    num_paths,
                    seed,
                    // Include mc_config when the field exists (i.e., when mc feature is enabled)
                    // The field is conditionally compiled in finstack-valuations with #[cfg(feature = "mc")]
                    mc_config: None,
                };
                DrawRepaySpec::Stochastic(spec)
            } else {
                return Err(PyValueError::new_err(
                    "draw_repay_spec must have 'deterministic' or 'stochastic' key"
                ));
            }
        } else {
            return Err(PyValueError::new_err(
                "draw_repay_spec must be a dict"
            ));
        };

        let mut builder = RevolvingCredit::builder();
        builder = builder.id(id);
        builder = builder.commitment_amount(commitment);
        builder = builder.drawn_amount(drawn);
        builder = builder.commitment_date(commit_date);
        builder = builder.maturity_date(mat_date);
        builder = builder.base_rate_spec(base_rate);
        builder = builder.payment_frequency(pay_freq);
        builder = builder.fees(fees_struct);
        builder = builder.draw_repay_spec(draw_repay);
        builder = builder.disc_id(disc_id);
        builder = builder.day_count(DayCount::Act365F);
        let rev_credit = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build RevolvingCredit: {e}"))
        })?;
        Ok(Self::new(rev_credit))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Commitment amount.
    #[getter]
    fn commitment_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.commitment_amount)
    }

    /// Drawn amount.
    #[getter]
    fn drawn_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.drawn_amount)
    }

    /// Commitment date.
    #[getter]
    fn commitment_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.commitment_date)
    }

    /// Maturity date.
    #[getter]
    fn maturity_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity_date)
    }

    fn __repr__(&self) -> String {
        format!(
            "RevolvingCredit(id='{}', commitment={}, drawn={})",
            self.inner.id.as_str(),
            self.inner.commitment_amount.amount(),
            self.inner.drawn_amount.amount()
        )
    }
}

fn parse_frequency(freq_str: Option<&str>) -> PyResult<finstack_core::dates::Frequency> {
    parse_frequency_label(freq_str)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "revolving_credit")?;
    module.add_class::<PyRevolvingCredit>()?;
    let exports = ["RevolvingCredit"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}

