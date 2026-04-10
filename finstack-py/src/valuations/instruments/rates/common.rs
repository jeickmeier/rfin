use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::collections::HashMap;

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

pub(crate) fn meta_attributes(pending_attributes: Option<&HashMap<String, String>>) -> Attributes {
    let mut attrs = Attributes::new();
    if let Some(pending) = pending_attributes {
        for (key, value) in pending {
            attrs.meta.insert(key.clone(), value.clone());
        }
    }
    attrs
}

pub(crate) fn require_builder_field<T>(
    builder_name: &str,
    field_name: &str,
    value: Option<T>,
) -> PyResult<T> {
    value.ok_or_else(|| builder_internal_missing(builder_name, field_name))
}

pub(crate) fn require_builder_clone<T: Clone>(
    builder_name: &str,
    field_name: &str,
    value: Option<&T>,
) -> PyResult<T> {
    value
        .cloned()
        .ok_or_else(|| builder_internal_missing(builder_name, field_name))
}

pub(crate) fn require_notional_money(
    builder_name: &str,
    amount: Option<f64>,
    currency: Option<Currency>,
) -> PyResult<Money> {
    require_money(builder_name, "notional", amount, currency)
}

pub(crate) fn require_money(
    builder_name: &str,
    amount_field_name: &str,
    amount: Option<f64>,
    currency: Option<Currency>,
) -> PyResult<Money> {
    let amount = require_builder_field(builder_name, amount_field_name, amount)?;
    let currency = require_builder_field(builder_name, "currency", currency)?;
    Ok(Money::new(amount, currency))
}

fn builder_internal_missing(builder_name: &str, field_name: &str) -> PyErr {
    PyRuntimeError::new_err(format!(
        "{builder_name} internal error: missing {field_name} after validation"
    ))
}
