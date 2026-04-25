//! Direct Python wrappers for FX valuation instruments.

use crate::bindings::extract::extract_market;
use crate::errors::display_to_py;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use serde_json::{Map, Value};

fn py_to_json_value<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>, label: &str) -> PyResult<Value> {
    if let Ok(json) = obj.extract::<String>() {
        return serde_json::from_str(&json)
            .map_err(|e| PyValueError::new_err(format!("invalid {label} JSON: {e}")));
    }

    let json_mod = py.import("json")?;
    let json: String = json_mod
        .call_method1("dumps", (obj,))
        .and_then(|value| value.extract())
        .map_err(|e| PyValueError::new_err(format!("invalid {label}: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| PyValueError::new_err(format!("invalid {label} JSON: {e}")))
}

fn canonical_payload(type_tag: &str, value: Value) -> PyResult<String> {
    let payload = if value.get("type").is_some() {
        let actual = value.get("type").and_then(Value::as_str).ok_or_else(|| {
            PyValueError::new_err("instrument JSON field `type` must be a string")
        })?;
        if actual != type_tag {
            return Err(PyValueError::new_err(format!(
                "expected instrument type `{type_tag}`, got `{actual}`"
            )));
        }
        value
    } else {
        let mut payload = Map::new();
        payload.insert("type".to_string(), Value::String(type_tag.to_string()));
        payload.insert("spec".to_string(), value);
        Value::Object(payload)
    };

    let json = serde_json::to_string(&payload).map_err(display_to_py)?;
    finstack_valuations::pricer::validate_instrument_json(&json).map_err(display_to_py)
}

fn build_from_py(
    py: Python<'_>,
    type_tag: &str,
    spec: Option<&Bound<'_, PyAny>>,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    if spec.is_some() && kwargs.is_some_and(|d| !d.is_empty()) {
        return Err(PyValueError::new_err(
            "pass either a spec object/JSON or keyword fields, not both",
        ));
    }

    let value = if let Some(spec) = spec {
        py_to_json_value(py, spec, "FX instrument spec")?
    } else if let Some(kwargs) = kwargs {
        py_to_json_value(py, kwargs.as_any(), "FX instrument keyword fields")?
    } else {
        return Err(PyValueError::new_err(
            "FX instrument constructor requires a spec object, JSON string, or keyword fields",
        ));
    };
    canonical_payload(type_tag, value)
}

fn from_json_payload(type_tag: &str, json: &str) -> PyResult<String> {
    let value: Value = serde_json::from_str(json).map_err(display_to_py)?;
    canonical_payload(type_tag, value)
}

fn pretty_json(json: &str) -> PyResult<String> {
    let value: Value = serde_json::from_str(json).map_err(display_to_py)?;
    serde_json::to_string_pretty(&value).map_err(display_to_py)
}

fn price_payload(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
) -> PyResult<String> {
    let market = extract_market(market)?;
    let result = finstack_valuations::pricer::price_instrument_json(json, &market, as_of, model)
        .map_err(display_to_py)?;
    serde_json::to_string_pretty(&result).map_err(display_to_py)
}

fn price_payload_with_metrics(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
    metrics: Vec<String>,
    pricing_options: Option<&str>,
) -> PyResult<String> {
    let market = extract_market(market)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        json,
        &market,
        as_of,
        model,
        &metrics,
        pricing_options,
    )
    .map_err(display_to_py)?;
    serde_json::to_string_pretty(&result).map_err(display_to_py)
}

fn metric_value(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
    metric: &str,
) -> PyResult<f64> {
    let market = extract_market(market)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        json,
        &market,
        as_of,
        model,
        &[metric.to_string()],
        None,
    )
    .map_err(display_to_py)?;
    result
        .metric_str(metric)
        .ok_or_else(|| PyValueError::new_err(format!("metric `{metric}` was not returned")))
}

macro_rules! fx_class {
    ($py_name:literal, $rust_name:ident, $type_tag:literal) => {
        #[pyclass(name = $py_name, module = "finstack.valuations.fx", skip_from_py_object)]
        #[derive(Clone)]
        pub(crate) struct $rust_name {
            json: String,
        }

        #[pymethods]
        impl $rust_name {
            #[new]
            #[pyo3(signature = (spec=None, **kwargs))]
            fn new(
                py: Python<'_>,
                spec: Option<&Bound<'_, PyAny>>,
                kwargs: Option<&Bound<'_, PyDict>>,
            ) -> PyResult<Self> {
                Ok(Self {
                    json: build_from_py(py, $type_tag, spec, kwargs)?,
                })
            }

            #[staticmethod]
            fn from_json(json: &str) -> PyResult<Self> {
                Ok(Self {
                    json: from_json_payload($type_tag, json)?,
                })
            }

            fn to_json(&self) -> PyResult<String> {
                pretty_json(&self.json)
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn price(
                &self,
                market: &Bound<'_, PyAny>,
                as_of: &str,
                model: &str,
            ) -> PyResult<String> {
                price_payload(&self.json, market, as_of, model)
            }

            #[pyo3(signature = (market, as_of, model="default", metrics=vec![], pricing_options=None))]
            fn price_with_metrics(
                &self,
                market: &Bound<'_, PyAny>,
                as_of: &str,
                model: &str,
                metrics: Vec<String>,
                pricing_options: Option<&str>,
            ) -> PyResult<String> {
                price_payload_with_metrics(
                    &self.json,
                    market,
                    as_of,
                    model,
                    metrics,
                    pricing_options,
                )
            }

            fn __repr__(&self) -> String {
                concat!($py_name, "(...)").to_string()
            }
        }
    };
}

macro_rules! fx_option_class {
    ($py_name:literal, $rust_name:ident, $type_tag:literal) => {
        fx_class!($py_name, $rust_name, $type_tag);

        #[pymethods]
        impl $rust_name {
            #[pyo3(signature = (market, as_of, model="default"))]
            fn delta(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "delta")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn gamma(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "gamma")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn vega(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "vega")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn theta(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "theta")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn rho(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "rho")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn foreign_rho(
                &self,
                market: &Bound<'_, PyAny>,
                as_of: &str,
                model: &str,
            ) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "foreign_rho")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn vanna(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "vanna")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn volga(&self, market: &Bound<'_, PyAny>, as_of: &str, model: &str) -> PyResult<f64> {
                metric_value(&self.json, market, as_of, model, "volga")
            }

            #[pyo3(signature = (market, as_of, model="default"))]
            fn greeks<'py>(
                &self,
                py: Python<'py>,
                market: &Bound<'_, PyAny>,
                as_of: &str,
                model: &str,
            ) -> PyResult<Bound<'py, PyDict>> {
                let out = PyDict::new(py);
                for metric in [
                    "delta",
                    "gamma",
                    "vega",
                    "theta",
                    "rho",
                    "foreign_rho",
                    "vanna",
                    "volga",
                ] {
                    if let Ok(value) = metric_value(&self.json, market, as_of, model, metric) {
                        out.set_item(metric, value)?;
                    }
                }
                Ok(out)
            }
        }
    };
}

fx_class!("FxSpot", PyFxSpot, "fx_spot");
fx_class!("FxForward", PyFxForward, "fx_forward");
fx_class!("FxSwap", PyFxSwap, "fx_swap");
fx_class!("Ndf", PyNdf, "ndf");
fx_option_class!("FxOption", PyFxOption, "fx_option");
fx_option_class!("FxDigitalOption", PyFxDigitalOption, "fx_digital_option");
fx_option_class!("FxTouchOption", PyFxTouchOption, "fx_touch_option");
fx_option_class!("FxBarrierOption", PyFxBarrierOption, "fx_barrier_option");
fx_class!("FxVarianceSwap", PyFxVarianceSwap, "fx_variance_swap");
fx_option_class!("QuantoOption", PyQuantoOption, "quanto_option");

/// Register the `finstack.valuations.fx` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "fx")?;
    m.setattr("__doc__", "Direct FX valuation instrument wrappers.")?;

    m.add_class::<PyFxSpot>()?;
    m.add_class::<PyFxForward>()?;
    m.add_class::<PyFxSwap>()?;
    m.add_class::<PyNdf>()?;
    m.add_class::<PyFxOption>()?;
    m.add_class::<PyFxDigitalOption>()?;
    m.add_class::<PyFxTouchOption>()?;
    m.add_class::<PyFxBarrierOption>()?;
    m.add_class::<PyFxVarianceSwap>()?;
    m.add_class::<PyQuantoOption>()?;

    let all = PyList::new(
        py,
        [
            "FxSpot",
            "FxForward",
            "FxSwap",
            "Ndf",
            "FxOption",
            "FxDigitalOption",
            "FxTouchOption",
            "FxBarrierOption",
            "FxVarianceSwap",
            "QuantoOption",
        ],
    )?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;
    parent.add("fx", &m)?;

    let parent_name: String = parent
        .getattr("__name__")
        .and_then(|attr| attr.extract())
        .unwrap_or_else(|_| "finstack.finstack.valuations".to_string());
    let qual = format!("{parent_name}.fx");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    sys.getattr("modules")?.set_item(&qual, &m)?;

    Ok(())
}
