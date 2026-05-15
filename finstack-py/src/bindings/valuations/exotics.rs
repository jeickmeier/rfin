//! Direct Python wrappers for exotic valuation instruments.

use crate::bindings::extract::extract_market_ref;
use crate::errors::display_to_py;
use finstack_valuations::pricer::{
    canonical_instrument_json, canonical_instrument_json_from_str,
    metric_value_from_instrument_json, present_standard_option_greeks_from_instrument_json,
    pretty_instrument_json, price_instrument_json, price_instrument_json_with_metrics,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use serde_json::Value;

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
        py_to_json_value(py, spec, "exotic instrument spec")?
    } else if let Some(kwargs) = kwargs {
        py_to_json_value(py, kwargs.as_any(), "exotic instrument keyword fields")?
    } else {
        return Err(PyValueError::new_err(
            "exotic instrument constructor requires a spec object, JSON string, or keyword fields",
        ));
    };
    canonical_instrument_json(type_tag, value).map_err(display_to_py)
}

fn from_json_payload(type_tag: &str, json: &str) -> PyResult<String> {
    canonical_instrument_json_from_str(type_tag, json).map_err(display_to_py)
}

fn pretty_json(json: &str) -> PyResult<String> {
    pretty_instrument_json(json).map_err(display_to_py)
}

fn price_payload(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
) -> PyResult<String> {
    let market = extract_market_ref(market)?;
    let result = price_instrument_json(json, &market, as_of, model).map_err(display_to_py)?;
    serde_json::to_string(&result).map_err(display_to_py)
}

fn price_payload_with_metrics(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
    metrics: Vec<String>,
    pricing_options: Option<&str>,
) -> PyResult<String> {
    let market = extract_market_ref(market)?;
    let result =
        price_instrument_json_with_metrics(json, &market, as_of, model, &metrics, pricing_options)
            .map_err(display_to_py)?;
    serde_json::to_string(&result).map_err(display_to_py)
}

fn metric_value(
    json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    model: &str,
    metric: &str,
) -> PyResult<f64> {
    let market = extract_market_ref(market)?;
    metric_value_from_instrument_json(json, &market, as_of, model, metric).map_err(display_to_py)
}

macro_rules! exotic_class {
    ($py_name:literal, $rust_name:ident, $type_tag:literal) => {
        #[pyclass(
            name = $py_name,
            module = "finstack.valuations.exotics",
            skip_from_py_object
        )]
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

macro_rules! exotic_option_class {
    ($py_name:literal, $rust_name:ident, $type_tag:literal) => {
        exotic_class!($py_name, $rust_name, $type_tag);

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
                let market = extract_market_ref(market)?;
                let pairs = present_standard_option_greeks_from_instrument_json(
                    &self.json, &market, as_of, model,
                )
                .map_err(display_to_py)?;
                for (metric, value) in pairs {
                    out.set_item(metric, value)?;
                }
                Ok(out)
            }
        }
    };
}

exotic_option_class!("AsianOption", PyAsianOption, "asian_option");
exotic_option_class!("BarrierOption", PyBarrierOption, "barrier_option");
exotic_option_class!("LookbackOption", PyLookbackOption, "lookback_option");
exotic_class!("Basket", PyBasket, "basket");

/// Register the `finstack.valuations.exotics` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "exotics")?;
    m.setattr("__doc__", "Direct exotic valuation instrument wrappers.")?;

    m.add_class::<PyAsianOption>()?;
    m.add_class::<PyBarrierOption>()?;
    m.add_class::<PyLookbackOption>()?;
    m.add_class::<PyBasket>()?;

    let all = PyList::new(
        py,
        ["AsianOption", "BarrierOption", "LookbackOption", "Basket"],
    )?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;
    parent.add("exotics", &m)?;

    let parent_name: String = parent.getattr("__name__")?.extract()?;
    let qual = format!("{parent_name}.exotics");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    sys.getattr("modules")?.set_item(&qual, &m)?;

    Ok(())
}
