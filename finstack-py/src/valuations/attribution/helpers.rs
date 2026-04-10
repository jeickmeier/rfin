use crate::core::market_data::term_structures::{
    PyBaseCorrelationCurve, PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use finstack_valuations::attribution::{AttributionFactor, ModelParamsSnapshot};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyString;
use pythonize::depythonize;
use serde_json::{self, Value};
use std::sync::Arc;

pub(super) fn parse_model_params_snapshot(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<ModelParamsSnapshot>> {
    if let Some(obj) = value {
        if obj.is_none() {
            return Ok(None);
        }

        if let Ok(text) = obj.cast::<PyString>() {
            let snapshot: ModelParamsSnapshot =
                serde_json::from_str(text.to_str()?).map_err(|err| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Invalid model_params_t0 JSON: {err}"
                    ))
                })?;
            return Ok(Some(snapshot));
        }

        let json_value: Value = depythonize(obj)?;
        let snapshot: ModelParamsSnapshot = serde_json::from_value(json_value).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "model_params_t0 does not match expected schema: {err}"
            ))
        })?;
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

pub(super) fn factor_to_label(factor: &AttributionFactor) -> &'static str {
    match factor {
        AttributionFactor::Carry => "carry",
        AttributionFactor::RatesCurves => "rates_curves",
        AttributionFactor::CreditCurves => "credit_curves",
        AttributionFactor::InflationCurves => "inflation_curves",
        AttributionFactor::Correlations => "correlations",
        AttributionFactor::Fx => "fx",
        AttributionFactor::Volatility => "volatility",
        AttributionFactor::ModelParameters => "model_parameters",
        AttributionFactor::MarketScalars => "market_scalars",
    }
}

pub(super) fn money_map_from_python(
    values: HashMap<String, crate::core::money::PyMoney>,
) -> IndexMap<CurveId, Money> {
    values
        .into_iter()
        .map(|(key, value)| (CurveId::new(key), value.inner))
        .collect()
}

pub(super) fn money_map_to_python(
    values: &IndexMap<CurveId, Money>,
) -> HashMap<String, crate::core::money::PyMoney> {
    values
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

pub(super) fn money_pair_map_to_python(
    values: &IndexMap<(CurveId, String), Money>,
) -> HashMap<(String, String), crate::core::money::PyMoney> {
    values
        .iter()
        .map(|((curve_id, tenor), value)| {
            (
                (curve_id.to_string(), tenor.clone()),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

pub(super) fn money_pair_map_from_python(
    values: HashMap<(String, String), crate::core::money::PyMoney>,
) -> IndexMap<(CurveId, String), Money> {
    values
        .into_iter()
        .map(|((curve_id, tenor), value)| ((CurveId::new(curve_id), tenor), value.inner))
        .collect()
}

pub(super) fn fx_pair_map_from_python(
    values: HashMap<(String, String), crate::core::money::PyMoney>,
) -> PyResult<IndexMap<(Currency, Currency), Money>> {
    values
        .into_iter()
        .map(|((from, to), value)| {
            Ok((
                (
                    Currency::try_from(from.as_str()).map_err(|_| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Unknown currency code in FX attribution: {from}"
                        ))
                    })?,
                    Currency::try_from(to.as_str()).map_err(|_| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Unknown currency code in FX attribution: {to}"
                        ))
                    })?,
                ),
                value.inner,
            ))
        })
        .collect()
}

pub(super) fn fx_pair_map_to_python(
    values: &IndexMap<(Currency, Currency), Money>,
) -> HashMap<(String, String), crate::core::money::PyMoney> {
    values
        .iter()
        .map(|((from, to), value)| {
            (
                (from.to_string(), to.to_string()),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

pub(super) fn wrap_discount_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::DiscountCurve>>,
) -> HashMap<String, PyDiscountCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyDiscountCurve::new_arc(curve.clone())))
        .collect()
}

pub(super) fn wrap_forward_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::ForwardCurve>>,
) -> HashMap<String, PyForwardCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyForwardCurve::new_arc(curve.clone())))
        .collect()
}

pub(super) fn wrap_hazard_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::HazardCurve>>,
) -> HashMap<String, PyHazardCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyHazardCurve::new_arc(curve.clone())))
        .collect()
}

pub(super) fn wrap_inflation_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::InflationCurve>>,
) -> HashMap<String, PyInflationCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyInflationCurve::new_arc(curve.clone())))
        .collect()
}

pub(super) fn wrap_base_correlation_curves(
    values: &HashMap<
        CurveId,
        Arc<finstack_core::market_data::term_structures::BaseCorrelationCurve>,
    >,
) -> HashMap<String, PyBaseCorrelationCurve> {
    values
        .iter()
        .map(|(key, curve)| {
            (
                key.to_string(),
                PyBaseCorrelationCurve::new_arc(curve.clone()),
            )
        })
        .collect()
}
