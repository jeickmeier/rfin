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
