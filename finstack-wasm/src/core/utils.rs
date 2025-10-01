use wasm_bindgen::JsValue;

/// Push values into a JavaScript array from an iterator.
pub(crate) fn js_array_from_iter<T>(items: impl IntoIterator<Item = T>) -> js_sys::Array
where
    T: Into<JsValue>,
{
    let array = js_sys::Array::new();
    for value in items {
        array.push(&value.into());
    }
    array
}

/// Convert a message into a JavaScript Error value.
/// 
/// Note: Also available via crate::core::error::js_error for consistency.
#[inline]
pub(crate) fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from(js_sys::Error::new(&message.into()))
}
