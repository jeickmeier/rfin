use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub(crate) struct FxOptionMarketContext {
    pub(crate) instrument_id: InstrumentId,
    pub(crate) base_currency: Currency,
    pub(crate) quote_currency: Currency,
    pub(crate) expiry: time::Date,
    pub(crate) domestic_discount_curve_id: CurveId,
    pub(crate) foreign_discount_curve_id: CurveId,
    pub(crate) vol_surface_id: CurveId,
    pub(crate) day_count: DayCount,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validated_option_context(
    builder_name: &str,
    instrument_id: &InstrumentId,
    base_currency: Option<Currency>,
    quote_currency: Option<Currency>,
    expiry: Option<time::Date>,
    domestic_discount_curve_id: Option<CurveId>,
    foreign_discount_curve_id: Option<CurveId>,
    vol_surface_id: Option<CurveId>,
    day_count: DayCount,
) -> PyResult<FxOptionMarketContext> {
    Ok(FxOptionMarketContext {
        instrument_id: instrument_id.clone(),
        base_currency: require_field(builder_name, "base_currency", base_currency)?,
        quote_currency: require_field(builder_name, "quote_currency", quote_currency)?,
        expiry: require_field(builder_name, "expiry", expiry)?,
        domestic_discount_curve_id: require_field(
            builder_name,
            "domestic curve",
            domestic_discount_curve_id,
        )?,
        foreign_discount_curve_id: require_field(
            builder_name,
            "foreign curve",
            foreign_discount_curve_id,
        )?,
        vol_surface_id: require_field(builder_name, "vol surface", vol_surface_id)?,
        day_count,
    })
}

pub(crate) fn default_pricing_overrides() -> PricingOverrides {
    PricingOverrides::default()
}

pub(crate) fn default_attributes() -> Attributes {
    Attributes::new()
}

pub(crate) fn required_value<T>(value: Option<T>, error_message: &'static str) -> PyResult<T> {
    value.ok_or_else(|| PyValueError::new_err(error_message))
}

pub(crate) fn required_clone<T: Clone>(
    value: Option<&T>,
    error_message: &'static str,
) -> PyResult<T> {
    value
        .cloned()
        .ok_or_else(|| PyValueError::new_err(error_message))
}

pub(crate) fn ensure_distinct_currencies(
    base_currency: Currency,
    quote_currency: Currency,
    error_message: &'static str,
) -> PyResult<()> {
    if base_currency == quote_currency {
        return Err(PyValueError::new_err(error_message));
    }
    Ok(())
}

pub(crate) fn ensure_notional_currency(
    notional: Money,
    expected_currency: Currency,
    error_prefix: &str,
) -> PyResult<Money> {
    if notional.currency() != expected_currency {
        return Err(PyValueError::new_err(format!(
            "{error_prefix} ({}) must match base_currency ({})",
            notional.currency(),
            expected_currency
        )));
    }
    Ok(notional)
}

pub(crate) fn ensure_positive(value: f64, error_message: &'static str) -> PyResult<f64> {
    if value <= 0.0 {
        return Err(PyValueError::new_err(error_message));
    }
    Ok(value)
}

pub(crate) fn ensure_after<T: Ord>(
    later: T,
    earlier: T,
    error_message: &'static str,
) -> PyResult<T> {
    if later <= earlier {
        return Err(PyValueError::new_err(error_message));
    }
    Ok(later)
}

fn require_field<T>(builder_name: &str, field_name: &str, value: Option<T>) -> PyResult<T> {
    value.ok_or_else(|| {
        PyRuntimeError::new_err(format!(
            "{builder_name} internal error: missing {field_name} after validation"
        ))
    })
}
