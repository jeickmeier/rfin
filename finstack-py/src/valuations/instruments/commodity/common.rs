use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

pub(crate) fn validated_field<T>(
    builder_name: &str,
    field_name: &str,
    value: Option<T>,
) -> PyResult<T> {
    value.ok_or_else(|| {
        PyRuntimeError::new_err(format!(
            "{builder_name} internal error: missing {field_name} after validation"
        ))
    })
}

pub(crate) fn validated_clone<T: Clone>(
    builder_name: &str,
    field_name: &str,
    value: Option<&T>,
) -> PyResult<T> {
    value.cloned().ok_or_else(|| {
        PyRuntimeError::new_err(format!(
            "{builder_name} internal error: missing {field_name} after validation"
        ))
    })
}

pub(crate) fn required_value<T>(value: Option<T>, error_message: &'static str) -> PyResult<T> {
    value.ok_or_else(|| PyValueError::new_err(error_message))
}

pub(crate) fn ensure_positive(value: f64, error_message: &'static str) -> PyResult<f64> {
    if value <= 0.0 {
        return Err(PyValueError::new_err(error_message));
    }
    Ok(value)
}

pub(crate) fn ensure_non_empty<T>(values: &[T], error_message: &'static str) -> PyResult<()> {
    if values.is_empty() {
        return Err(PyValueError::new_err(error_message));
    }
    Ok(())
}

pub(crate) fn option_pricing_overrides(
    implied_volatility: Option<f64>,
    tree_steps: Option<usize>,
) -> PricingOverrides {
    let mut pricing_overrides = PricingOverrides::default();
    if let Some(vol) = implied_volatility {
        pricing_overrides.market_quotes.implied_volatility = Some(vol);
    }
    if let Some(steps) = tree_steps {
        pricing_overrides.model_config.tree_steps = Some(steps);
    }
    pricing_overrides
}

pub(crate) fn default_pricing_overrides() -> PricingOverrides {
    PricingOverrides::default()
}

pub(crate) fn default_attributes() -> Attributes {
    Attributes::new()
}
