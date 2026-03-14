pub(crate) mod parameters;
pub(crate) mod parse;

use finstack_core::types::{CurveId, InstrumentId};

pub(crate) fn instrument_id_from_str(id: &str) -> InstrumentId {
    InstrumentId::new(id)
}

pub(crate) fn curve_id_from_str(id: &str) -> CurveId {
    CurveId::new(id)
}

pub(crate) fn optional_static_str(value: Option<String>) -> Option<&'static str> {
    value.map(|s| Box::leak(s.into_boxed_str()) as &'static str)
}

/// Convert an `f64` to [`rust_decimal::Decimal`], returning a [`wasm_bindgen::JsValue`] error
/// for non-finite or unrepresentable values.
///
/// This prevents silent masking of `NaN`/`Infinity` as zero, which would
/// produce zero rates or strikes and materially misprice instruments.
pub(crate) fn f64_to_decimal(
    value: f64,
    field: &str,
) -> Result<rust_decimal::Decimal, wasm_bindgen::JsValue> {
    finstack_valuations::utils::decimal::f64_to_decimal(value, field)
        .map_err(|e| crate::core::error::core_to_js(e))
}

/// Convert an `Option<f64>` to [`rust_decimal::Decimal`], using zero for `None`
/// and returning an error only when `Some(value)` cannot be converted.
///
/// Use this where `None` is documented to mean "no spread / zero spread" but a
/// provided non-finite value should still be rejected.
pub(crate) fn opt_f64_to_decimal(
    value: Option<f64>,
    field: &str,
) -> Result<rust_decimal::Decimal, wasm_bindgen::JsValue> {
    match value {
        None => Ok(rust_decimal::Decimal::ZERO),
        Some(v) => f64_to_decimal(v, field),
    }
}
